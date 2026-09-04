How to compile uboot with cross-compilation:

```
$ make distclean
$ make CROSS_COMPILE=riscv64-linux-gnu- starfive_visionfive2_defconfig
$ make olddefconfig
```
