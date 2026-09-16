## OS Flash Log — Orange Pi RV

**Board:** Orange Pi RV, fresh unit, no eMMC (SD only)
**Image:** `reptilian-riscv` `orangepi-rv` release, hash verified

### Issue
SD card flashes fine, U-Boot boots it fine, but Linux can't see any partitions once it takes over. Drops to initramfs, missing root UUID.

### Ruled out
- Corrupted download
- Bad write
- Bad card / bad board (manufacturer Debian boots clean on same setup)
- USB as workaround (crashes worse than SD)

### Likely cause (according to Claude)
`os.img` (~3GB) gets `dd`'d onto a bigger card with no resize after. GPT backup header ends up in the wrong spot for the real disk size, kernel rejects the table.

### Suggested fix
Add `sgdisk -e $DEVICE` (or `growpart`) after the `dd` step in `flash-os.sh`. (Theory proposed by Claude)

### Status
Hardware confirmed working. Manufacture linked Debian working.

*(Debugged with Claude)*