# dm-verity setup

## What this does

dm-verity makes the exam root filesystem **read-only and hash-checked**. A copy of the live root is written to a data partition and a SHA-256 tree of that data is written to a second (hash) partition. At boot, initramfs opens `/dev/mapper/verityroot` only if the data still matches the saved **root hash**. The kernel then mounts that mapper as `/`.

A student who changes files on that root will fail the check on the next boot (or cannot persist the change). The writable “dev” root is left alone so you can still update the image and repeat the format.

This is **not** in the CI image or `scripts/flash-os.sh`. A flashed board still boots a normal writable Debian root until these steps are done. It also does not lock U-Boot or SPI flash; that is separate signed-boot work.

Summary of notes from Nick on the Orange Pi RV.

Do this **on the board** (SSH or serial). The only files from this repo that go on the board are the two initramfs scripts in `scripts/verity/`. Do not run those scripts by hand.

| File in this repo | Destination on the board | When it runs |
|---|---|---|
| `scripts/verity/veritysetup` | `/etc/initramfs-tools/hooks/veritysetup` | During `update-initramfs` (copies binaries into the ramdisk) |
| `scripts/verity/verityroot` | `/etc/initramfs-tools/scripts/local-top/verityroot` | At boot, before the real root mounts |

---

## Prerequisites

- The board already boots Debian from the SD card.
- `veritysetup` (`cryptsetup-bin`), `rsync`, and `mkimage` (`u-boot-tools`) are installed.
- A **data** partition and a **hash** partition already exist. The notes use the devices below. How those extra partitions were created is not documented. Confirm with `lsblk` before running anything that wipes them.

| Role | Device in the notes |
|---|---|
| Writable root (copy **from**) | `/dev/mmcblk0p6` |
| Verity data (copy **to**) | `/dev/mmcblk0p7` |
| Verity hash tree | `/dev/mmcblk0p9` |

- `scripts/verity/verityroot` opens `/dev/disk/by-partlabel/verityrootfs` and `/dev/disk/by-partlabel/verityhash`. The format step only runs `mkfs.ext4 -L verityrootfs`, which is a filesystem label, not a GPT partition name. Those `by-partlabel` paths have to exist before a verity boot, or the paths in `verityroot` have to be changed. The notes do not say which was done.
- Edit `ROOT_HASH=` in `verityroot` to the hash from step 2 **before** installing that script. The value already in the file is from one earlier format and will not match a new one.
- Step 5 needs `kernel-signed.its` and a `keys/` directory. Neither is in this repo.

---

## 1. Copy the live root onto the data partition

This wipes the data partition. It copies everything on the source root. `rsync` of a full root takes a long time.

```bash
sudo mkfs.ext4 -L verityrootfs /dev/mmcblk0p7

sudo mkdir -p /mnt/opirv-verity /mnt/opirv-root
sudo mount /dev/mmcblk0p7 /mnt/opirv-verity
sudo mount /dev/mmcblk0p6 /mnt/opirv-root
sudo rsync -aHAX --numeric-ids /mnt/opirv-root/ /mnt/opirv-verity/

sudo umount /mnt/opirv-verity /mnt/opirv-root
```

## 2. Build the hash tree and save the root hash

```bash
sudo veritysetup format /dev/mmcblk0p7 /dev/mmcblk0p9
```

Save the **Root hash**. Put it in `scripts/verity/verityroot` (`ROOT_HASH=`). A new format produces a new hash.

The notes say the hash also belongs in the signed boot chain (`kernel.itb`), the initramfs hook, or the bootargs.

## 3. Test the mapper before changing boot

Stay on the writable root. If this fails, stop.

```bash
sudo veritysetup open /dev/mmcblk0p7 verityroot /dev/mmcblk0p9 <ROOT_HASH>
sudo mkdir -p /mnt/veritytest
sudo mount -o ro /dev/mapper/verityroot /mnt/veritytest
ls /mnt/veritytest
sudo umount /mnt/veritytest
sudo veritysetup close verityroot
```

Expect a normal root (`bin`, `etc`, `usr`, …).

## 4. Install the initramfs hook and local-top script

Both files must be executable.

```bash
sudo install -m 0755 scripts/verity/veritysetup /etc/initramfs-tools/hooks/veritysetup
sudo install -m 0755 scripts/verity/verityroot /etc/initramfs-tools/scripts/local-top/verityroot
```

Add to `/etc/initramfs-tools/modules` if missing:

```
dm_mod
dm_verity
```

Rebuild initramfs and the U-Boot ramdisk. The notes use kernel `5.15.0-starfive2`. Use `uname -r` if that is not the running kernel.

```bash
sudo update-initramfs -u -k 5.15.0-starfive2
sudo mkimage -A riscv -O linux -T ramdisk -C gzip \
  -n uInitrd \
  -d /boot/initrd.img-5.15.0-starfive2 \
  /boot/uInitrd
```

## 5. Rebuild the signed `kernel.itb`

The notes assume keys are already present and the U-Boot signature is already wired:

```bash
mkimage \
  -f kernel-signed.its \
  -k keys \
  -r \
  kernel.itb
```

The notes also say to modify SPI flash so the board mounts the verity partition instead of the writable root. How that FIT is installed, and the SPI flash steps, are not in these notes.

## 6. Point U-Boot at the mapper

Bootargs need to mount verity instead of the writable root. What worked:

```
console=tty0 console=ttyS0,115200 earlycon rootwait ro root=/dev/mapper/verityroot
```

The notes say to change U-Boot bootargs. They do not say whether that is the U-Boot environment, the FIT, or both.

## 7. Confirm after reboot

`dmesg` should include:

```
device-mapper: verity: sha256 using implementation "sha256-generic"
```

`/` should be `/dev/mapper/verityroot`. A wrong hash fails the open.

---

This does not lock SPI or U-Boot, and it is not part of the `reptilian-riscv` image build.
