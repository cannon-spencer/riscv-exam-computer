Here's how I made the verity partition, in this case on p7:

```
$ mkfs.ext4 -L verityrootfs /dev/mmcblk0p7
$ mount /dev/mmcblk0p7 /mnt/opirv-verity
$ mount /dev/mmcblk0p6 /mnt/opirv-root
$ rsync -aHAX --numeric-ids /mnt/opirv-root/ /mnt/opirv-verity/
```
This will copy all the contents of the devroot (e.g. p6) to p7. So, make sure there isn't any sensitive stuff there.
It will also take literally forever. Be prepared to wait for a looong time

Then, unmount the partitions and run this command:
```
$ veritysetup format /dev/mmcblk0p7 /dev/mmcblk0p9
```

This will output a root hash. Put that in the signed boot chain (kernel.itb), an initramfs hook, or the bootargs
