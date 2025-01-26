if [ ${EUID} -ne 0 ]; then
  echo "this script must be run as root" >&2
  exit 1
fi

if [ $# -lt 1 ]; then
  echo "usage: build_arch_only.sh <rootfs_dir>"
  exit 1
fi

cp bootloader.img terra_arch.img
truncate -s 4G terra_arch.img
OUT_DEV=$(losetup -Pf --show terra_arch.img)
# slep for a bit to allow the partition table to update and let the sys chill out (without this partprobe broke for me ;-;)
sleep 1
# just in damn case
partprobe "${OUT_DEV}"
(
echo "n"
echo "4"
echo ""
echo ""
echo ""
echo "t"
echo "4"
echo "3cb8e202-3b7e-47dd-8a3c-7ff2a13cfcec"
echo "w"
) | fdisk "${OUT_DEV}"
cgpt add -i 4 -l terra_arch "${OUT_DEV}"

mkfs.ext4 "${OUT_DEV}"p4

mkdir mnt
mount "${OUT_DEV}"p4 mnt 
cp -a "${1}"/* mnt/
umount mnt
rm -r mnt

losetup -d ${OUT_DEV}

# i hate zippery >:[ not useful for debugging rn until i make a prompt for it
# TODO: prompt user for zipping, if timeout, just do it:tm:
# zstd -k terra_arch.img
# zip terra_arch.img.zip terra_arch.img
