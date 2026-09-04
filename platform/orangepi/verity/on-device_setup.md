You'll notice there's a file here named `veritysetup`. That's a script for initramfs. It should be placed at:
`etc/initramfs-tools/scripts/local-top/verityroot`.

This is the step that wires dm-verity into initramfs. Then, you rebuild the kernel and modify spi flash to mount and use the immutable verity partition instead of the rootfs used for development.

It goes without saying, but make sure the script has executable permissions.

On the device, once you've moved the file in, regenerate initramfs:

```
$ sudo update-initramfs -u -k 5.15.0-starfive2
$ sudo mkimage -A riscv -O linux -T ramdisk -C gzip \
  -n uInitrd \
  -d /boot/initrd.img-5.15.0-starfive2 \
  /boot/uInitrd
```

Also, include the following in `/etc/initramfs-tools/modules` for good measure:
```
dm_mod
dm_verity
```

Next, there's another executable file here, `veritysetup`. That should be placed at `/etc/initramfs-tools/hooks/veritysetup`. It copies the verity-related binaries into initramfs so it can make use of them for stuff.

Then, rebuild the (signed) `kernel.itb`. At this point, the uboot signature is assumed to be embedded, so we use the simpler command:

```
$ mkimage \
  -f kernel-signed.its \
  -k keys \
  -r \
  kernel.itb
```

During the next boot, you'll probably have to modify the bootargs for uboot so it knows to mount the verity partition instead of the dev rootfs.
