if [ ${EUID} -ne 0 ]; then
  echo "this script must be run as root" >&2
  exit 1
fi

if [ $# -lt 2 ]; then
  echo "usage: build_arch_only.sh <rootfs_dir> <bootloader_img>"
  exit 1
fi

cp "${2}" terra_arch.img
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

losetup -d "${OUT_DEV}"
echo "Do you want to zip the image? (y/n) (Auto-zipping in 5 seconds...)"
read -r -t 5 -n 1 user_input

if [[ "$user_input" == "y" ]]; then
  echo -e "\nZipping..."
  zstd -k terra_arch.img
  zip terra_arch.img.zip terra_arch.img
elif [[ "$user_input" == "n" ]]; then
  echo -e "\nSkipping zipping."
else
  echo -e "\nNo input detected. Auto-zipping..."
  zstd -k terra_arch.img
  zip terra_arch.img.zip terra_arch.img
fi
