You should run this command with the boot partition of the sd card mounted at `/mnt/opirv-boot`.

```
$ sudo mkimage \
  -f /mnt/opirv-boot/kernel-signed.its \
  -k /mnt/opirv-boot/keys \
  -K ./dts/dt.dtb \
  -r \
  /mnt/opirv-boot/kernel.itb
```

Then, check that the key showed up. You should see a "signature" node.

`$ fdtdump ./dts/dt.dtb | grep -i -A 15 signature`

Next, rebuild again so `CONFIG_OF_EMBED` pulls the modified `.dts/dt.dtb` into uboot:

`$ make -j"$(nproc)" CROSS_COMPILE=riscv64-linux-gnu-`

Then, copy `visionfive2_fw_payload.img` to the orangepi and proceed with the spi flashing.
