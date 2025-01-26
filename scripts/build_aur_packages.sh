git clone https://aur.archlinux.org/systemd-chromiumos
cd systemd-chromiumos
makepkg --skippgpcheck -s
cp *.pkg.tar.zst ..
cd ..
git clone https://aur.archlinux.org/yay
cd yay
makepkg -s
cp *.pkg.tar.zst ..
cd ..
