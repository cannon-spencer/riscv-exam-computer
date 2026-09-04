Here are the environment variables used during boot. These are stored on SPI flash, so they need to be modified manually when configuring a new device (not on the SD card, in other words)

You can modify them during the uboot prompt.

```
StarFive # env print bootcmd
bootcmd=env run boot_fit
StarFive # env print boot_fit
boot_fit=mmc dev 1; load mmc 1:5 ${loadaddr} /kernel.itb && iminfo ${loadaddr} && imxtract ${loadaddr} kernel ${kernel_addr_r} && imxtract ${loadaddr} fdt ${fdt_addr_r} && imxtract ${loadaddr} ramdisk ${ramdisk_addr_r} && booti ${kernel_addr_r} ${ramdisk_addr_r}:${filesize} ${fdt_addr_r}
StarFive # env print bootargs
bootargs=root=UUID=a4acd86b-e683-4784-a199-20ec3018a088 console=tty0 console=ttyS0,115200 earlycon rootwait
StarFive # env print kernel_addr_r
kernel_addr_r=0x40200000
StarFive # env print ramdisk_addr_r
ramdisk_addr_r=0x46100000
StarFive # env print fdt_addr_r
fdt_addr_r=0x46000000
StarFive # env print loadaddr
loadaddr=0x60000000
StarFive # env print fdt_high
fdt_high=0xffffffffffffffff
StarFive # env print initrd_high
initrd_high=0xffffffffffffffff
StarFive #
```

These may not exactly match what you need to use (e.g. UUID).

You can modify them over UART using `env set` or `env edit`.
Save them to spi flash using `env save`.

Theoretically, you could avoid having to modify these through the uboot prompt. Instead, just pull the mtd from the working device in its entirety once you're in linux proper with `dd` and flash it to the new device. The exact commands are an exercise left for the reader.

** IF USING VERITY

If you're using verity, then you DEFINITELY don't want bootargs to be using that uuid (the mutable rootfs for development). Instead, make bootargs the following:
```
console=tty0 console=ttyS0,115200 earlycon rootwait ro root=/dev/mapper/verityroot
```

Unfortunately, the easiest way to switch between these seems to be to just modify bootargs and type in the UUID or verity stuff after interrupting the autoboot sequence. I tried a more programmatic approach, but uboot shell syntax is weird. Feel free to give that a shot if you're interested.
