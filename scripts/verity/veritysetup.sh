#!/bin/sh
# initramfs-tools hook. Copy to /etc/initramfs-tools/hooks/veritysetup
# and chmod +x. update-initramfs runs this; do not run it by hand.
#
# Puts veritysetup and dmsetup into the ramdisk so local-top/verityroot
# can open /dev/mapper/verityroot at boot.
set -e

PREREQ=""
prereqs() { echo "$PREREQ"; }

case "$1" in
  prereqs)
    prereqs
    exit 0
    ;;
esac

. /usr/share/initramfs-tools/hook-functions

copy_exec /usr/sbin/veritysetup /sbin/veritysetup
copy_exec /usr/sbin/dmsetup /sbin/dmsetup
