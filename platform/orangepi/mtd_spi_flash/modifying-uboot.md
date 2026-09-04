Check which mtc to flash with
`$ cat /proc/mtd`

Then flash (e.g. on mtd1)
The image file should be generated when you compile uboot. move it here

```
$ sync
$ sudo flashcp -v visionfive2_fw_payload.img /dev/mtd1
$ sync
```
