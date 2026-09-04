After successfully creating the verity image and obtaining the root hash, you should save it somewhere safe. Then, testing that it works:

```
$ sudo veritysetup open /dev/mmcblk0p7 verityroot /dev/mmcblk0p9 <ROOT_HASH>
$ sudo mkdir -p /mnt/veritytest
$ sudo mount -o ro /dev/mapper/verityroot /mnt/veritytest
$ ls /mnt/veritytest
```

Hopefully, you'll see the typical root directories. Nice! Then, unmount:

```
$ sudo umount /mnt/veritytest
$ sudo veritysetup close verityroot
```
