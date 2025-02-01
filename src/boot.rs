use crate::screens; // Adjust if `common` is nested under `screens`
use crate::tui;
use crate::utils;
use crate::TERMIOS_BACKUP;
use nix::mount;
use nix::sys::stat;
use nix::unistd;
use std::ffi::CString;
use std::fs;
use std::io::{self, Read, Write};
use std::os::linux::fs::MetadataExt;
pub fn update_status(message: &str, termsize: tui::Point, center: tui::Point) {
    // Clear the area where the status message will go, but not the box itself
    tui::move_cursor(tui::Point {
        row: center.row + 1,                          // Just below the "Mounting" text
        col: center.col - (message.len() / 2) as u16, // Center the message
    });

    // Clear the current line where the message is
    print!("\x1b[2K"); // Clears the current line

    // Print the new status message
    print!("{}", message);
    tui::flush();

    // Redraw the box to ensure it's intact
    tui::draw_box(tui::Point { row: 0, col: 0 }, termsize);
    tui::flush();
}

pub fn boot_from_partition(dev: String, termsize: tui::Point, init_cmd: &str) {
    let center = tui::get_center(tui::Point { row: 0, col: 0 }, termsize);
    tui::clear();
    tui::draw_box(tui::Point { row: 0, col: 0 }, termsize);

    tui::move_cursor(tui::Point {
        row: center.row,
        col: center.col - 8 / 2,
    });
    print!("Mounting");

    screens::common::show_usb_disclaimer(termsize);

    tui::flush();

    update_status("Requesting filesystem check...", termsize, center);
    // Center the prompt
    let prompt = "Do you want to check the partition for errors? (y/n): ";
    let prompt_len = prompt.len() as u16;
    let prompt_center = tui::Point {
        row: center.row + 2, // Move the prompt down a bit from the center
        col: center.col - prompt_len / 2,
    };
    tui::move_cursor(prompt_center);
    print!("{}", prompt);

    io::stdout().flush().expect("Failed to flush stdout");

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input");
    let input = input.trim().to_lowercase();

    if input == "y" || input == "yes" {
        update_status("Checking the filesystem...", termsize, center);
        utils::run_fsck(&dev).expect("Failed to check partition for errors");
    }

    update_status("Creating /newroot...", termsize, center);
    unistd::mkdir("/newroot", stat::Mode::S_IRWXU).expect("Failed to create newroot");
    // we use /bin/mount and not nix::mount::* because /bin/mount has autodetection of fstype

    update_status("Mounting partition to /newroot...", termsize, center);
    utils::run_cmd("/bin/mount", &[dev, "/newroot".into()]).expect("Failed to mount partition");

    boot_from_newroot(termsize, center, false, init_cmd);
}

pub fn boot_from_squashfs(file: String, termsize: tui::Point, init_cmd: &str) {
    let center = tui::get_center(tui::Point { row: 0, col: 0 }, termsize);
    tui::clear();
    tui::draw_box(tui::Point { row: 0, col: 0 }, termsize);

    tui::move_cursor(tui::Point {
        row: center.row,
        col: center.col - 7 / 2,
    });
    print!("Copying");
    tui::flush();

    unistd::mkdir("/newroot_tmpfs", stat::Mode::S_IRWXU).expect("Failed to create newroot_tmpfs");
    let file_metadata = fs::metadata(file.clone()).expect("Failed to get metadata of squashfs");
    let file_size = (file_metadata.st_size() / (1024 * 1024)) + 16;
    mount::mount(
        Some("tmpfs"),
        "/newroot_tmpfs",
        Some("tmpfs"),
        mount::MsFlags::empty(),
        Some(format!("size={}M", file_size).as_str()),
    )
    .expect("Failed to mount tmpfs");
    fs_extra::file::copy_with_progress(
        file,
        "/newroot_tmpfs/rootfs.squashfs",
        &fs_extra::file::CopyOptions::new().buffer_size(1000 * 1000),
        |progress| {
            let prog = (progress.copied_bytes * 100) / progress.total_bytes;
            tui::draw_progress(
                tui::Point {
                    row: center.row + 1,
                    col: center.col - 27 / 2,
                },
                prog.try_into().unwrap(),
            );
            tui::flush();
        },
    )
    .expect("Failed to copy the rootfs to a tmpfs");

    tui::clear();
    tui::draw_box(tui::Point { row: 0, col: 0 }, termsize);

    tui::move_cursor(tui::Point {
        row: center.row,
        col: center.col - 8 / 2,
    });
    print!("Mounting");
    tui::flush();

    unistd::mkdir("/work", stat::Mode::S_IRWXU).expect("Failed to create work");
    mount::mount(
        Some("tmpfs"),
        "/work",
        Some("tmpfs"),
        mount::MsFlags::empty(),
        Some("size=512M"),
    )
    .expect("Failed to mount /work");
    unistd::mkdir("/work/upper", stat::Mode::S_IRWXU).expect("Failed to create work/upper");
    unistd::mkdir("/work/lower", stat::Mode::S_IRWXU).expect("Failed to create work/lower");

    utils::run_cmd(
        "/bin/squashfuse",
        &["/newroot_tmpfs/rootfs.squashfs", "/work/lower"],
    )
    .expect("Failed to mount squashfs");
    unistd::mkdir("/newroot", stat::Mode::S_IRWXU).expect("Failed to create newroot");
    utils::run_cmd(
        "/bin/unionfs",
        &[
            "-o",
            "allow_other,suid,dev",
            "-o",
            "cow,chroot=/work,max_files=32768",
            "/lower=RO:/upper=RW",
            "/newroot",
        ],
    )
    .expect("Failed to mount unionfs");

    boot_from_newroot(termsize, center, true, init_cmd);
}

fn boot_from_newroot(termsize: tui::Point, center: tui::Point, keep_oldroot: bool, init_cmd: &str) {
    tui::clear();
    tui::draw_box(tui::Point { row: 0, col: 0 }, termsize);

    tui::move_cursor(tui::Point {
        row: center.row,
        col: center.col - 7 / 2,
    });
    print!("Booting");
    screens::common::show_usb_disclaimer(termsize);
    tui::flush();

    if keep_oldroot {
        unistd::mkdir("/newroot/oldroot", stat::Mode::S_IRWXU)
            .expect("Failed to create oldroot mountpoint");
    }

    let mut is_chromeos = false;

    update_status("Checking for ChromeOS...", termsize, center);
    if let Ok(mut lsb_release) = fs::File::open("/newroot/etc/lsb-release") {
        let mut lsb_release_contents = String::new();
        let _ = lsb_release.read_to_string(&mut lsb_release_contents);
        is_chromeos = lsb_release_contents.to_lowercase().contains("chromeos");
    }

    update_status("Unmounting /sys...", termsize, center);
    mount::umount2("/sys", mount::MntFlags::MNT_DETACH).expect("Failed to unmount /sys");
    if is_chromeos {
        update_status("Moving /dev to /newroot/dev...", termsize, center);
        mount::mount::<str, str, str, str>(
            Some("/dev"),
            "/newroot/dev",
            None,
            mount::MsFlags::MS_MOVE,
            None,
        )
        .expect("Failed to move /dev to /newroot/dev");
    } else {
        update_status("Unmounting /dev...", termsize, center);
        mount::umount2("/dev", mount::MntFlags::MNT_DETACH).expect("Failed to unmount /dev");
    }

    update_status("Unmounting /data...", termsize, center);
    mount::umount2("/data", mount::MntFlags::MNT_DETACH).expect("Failed to unmount /data");
    update_status("Unmounting /proc...", termsize, center);
    mount::umount2("/proc", mount::MntFlags::MNT_DETACH).expect("Failed to unmount /proc");

    if !keep_oldroot {
        update_status("Changing directory to /newroot...", termsize, center);
        unistd::chdir("/newroot").expect("Failed to chdir() to /newroot");
        update_status("pivot_root'ing...", termsize, center);
        unistd::pivot_root(".", ".").expect("Failed to pivot_root");
        update_status("Unmounting old root...", termsize, center);
        mount::umount2(".", mount::MntFlags::MNT_DETACH).expect("Failed to unmount old root");
    } else {
        update_status("pivot_rooting...", termsize, center);
        unistd::pivot_root("/newroot", "/newroot/oldroot").expect("Failed to pivot_root");
    }

    unsafe {
        if let Some(termios) = TERMIOS_BACKUP {
            let _ = tui::destroy_term(termios);
        }
    }

    tui::clear();
    tui::draw_box(tui::Point { row: 0, col: 0 }, termsize);

    tui::move_cursor(tui::Point {
        row: center.row,
        col: center.col - 7 / 2,
    });
    print!("Booting");
    update_status("All set and ready to boot!", termsize, center);
    screens::common::show_usb_disclaimer(termsize);
    tui::flush();

    let cmdline = [CString::new(init_cmd).expect("Failed to create cstr")];
    unistd::execv(&cmdline[0], &cmdline).unwrap();
    panic!("failed to exec?");
}
