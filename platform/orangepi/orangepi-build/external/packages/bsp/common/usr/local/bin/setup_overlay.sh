#!/bin/bash
set -e

source /etc/orangepi-release

if [[ "$DISTRIBUTION_CODENAME" =~ "bookworm"|"bullseye" ]]; then
	cat <<-'EOF' > /usr/share/initramfs-tools/hooks/custom
	cp /bin/grep "${DESTDIR}"/bin
	cp /bin/mount "${DESTDIR}"/bin
	cp /lib/aarch64-linux-gnu/libmount.so.1 "${DESTDIR}"/lib
	cp /lib/aarch64-linux-gnu/libpcre.so.3 "${DESTDIR}"/lib 2>/dev/null || true
	EOF
	chmod +x /usr/share/initramfs-tools/hooks/custom
	export PATH=$PATH:/usr/sbin
	update-initramfs -u > /dev/null 2>&1
fi

FLAG_FILE=/var/lib/overlay_done

if [ -f $FLAG_FILE ]; then
    exit 0
fi

ROOT_PART=$(findmnt -n -o SOURCE /)
echo "ROOT_PART=$ROOT_PART"

DISK=$(lsblk -no pkname "$ROOT_PART")
DISK="/dev/$DISK"
echo "DISK=$DISK"

PART="${DISK}p2"
echo "WRITE PART=$PART"

parted -s "$DISK" resizepart 2 100%
partprobe "$DISK"
sleep 1

echo "Expand EXT4..."
e2fsck -f "$PART" -y || true
sleep 1
resize2fs "$PART"

UUID=$(blkid -s UUID -o value "$PART")

#sed -i "s/^overlayroot=.*/overlayroot=\"\"/" /etc/overlayroot.conf
sed -i "s|^overlayroot=.*|overlayroot=\"device:dev=UUID=$UUID\"|" /etc/overlayroot.conf

touch $FLAG_FILE

echo "[OK].."

reboot
