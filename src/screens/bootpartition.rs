use crate::*;
use std::os::unix::fs::FileTypeExt;

fn get_custom_partition(termsize: tui::Point) -> Option<String> {
    tui::clear();
    tui::draw_box(tui::Point { row: 0, col: 0 }, termsize);

    let prompt = "Enter partition path (e.g., /dev/sdb4):";
    let input_row = termsize.row / 2 + 1; // Row for user input

    tui::move_cursor(tui::Point {
        row: termsize.row / 2,
        col: (termsize.col / 2) as u16 - (prompt.len() as u16 / 2),
    });
    print!("{}", prompt);

    tui::flush();

    let mut input = String::new();

    loop {
        // Re-draw the input line
        tui::move_cursor(tui::Point {
            row: input_row,
            col: (termsize.col / 2) as u16 - (input.len() as u16 / 2),
        });
        print!("{}\u{1b}[K", input); // Display input and clear the rest of the line

        tui::flush();

        match tui::read_char() {
            '\n' => {
                // User pressed Enter
                break;
            }
            '\x08' | '\u{7f}' => {
                // Handle Backspace
                input.pop();
            }
            c => {
                // Append any other character to input
                input.push(c);
            }
        }
    }

    // Validate the input
    if validate_partition_path(&input) {
        Some(input)
    } else {
        // Show an error message if invalid
        tui::move_cursor(tui::Point {
            row: input_row + 1,
            col: (termsize.col / 2) as u16 - 15,
        });
        print!("Invalid partition path.");
        tui::flush();
        None
    }
}
use std::fs;

fn validate_partition_path(path: &str) -> bool {
    if let Ok(metadata) = fs::metadata(path) {
        metadata.file_type().is_block_device() // Check if it's a block device
    } else {
        false
    }
}

pub fn show_screen(termsize: tui::Point) {
    let center = tui::get_center(tui::Point { row: 0, col: 0 }, termsize);

    tui::clear();
    tui::draw_box(tui::Point { row: 0, col: 0 }, termsize);

    tui::move_cursor(tui::Point {
        row: center.row,
        col: center.col - 31 / 2,
    });
    print!("Searching for rootfs partitions");

    screens::common::show_usb_disclaimer(termsize);
    screens::common::show_boot_keybinds(termsize);

    tui::flush();

    let parts = disks::scan_for_usable_root_partitions();
    let parts_readable: Vec<String> = parts
        .clone()
        .into_iter()
        .map(|d| format!("{}: {}", d.partition, d.label))
        .collect();
    let parts_len = parts.len();

    tui::clear();
    tui::draw_box(tui::Point { row: 0, col: 0 }, termsize);

    tui::move_cursor(tui::Point {
        row: 4,
        col: center.col - 21 / 2,
    });
    print!("Boot from a partition");

    let mut selected: usize = 0;
    let mut selected_option = false;
    let mut init_cmd = "/sbin/init";
    loop {
        tui::draw_box(
            tui::Point { row: 6, col: 4 },
            tui::Point {
                row: termsize.row - 4,
                col: termsize.col - 3,
            },
        );

        screens::common::show_usb_disclaimer(termsize);
        screens::common::show_boot_keybinds(termsize);

        show_selector(
            tui::Point { row: 7, col: 5 },
            ((termsize.row - 6 - 2) / 2).into(),
            parts_readable.clone(),
            selected,
        );

        tui::flush();

        match tui::read_char() {
            '\x1b' => {
                // Escape sequence for arrow keys
                tui::read_char();
                selected = match tui::read_char() {
                    'A' => clamp(selected - 1, 0, parts_len - 1), // Up arrow
                    'B' => clamp(selected + 1, 0, parts_len - 1), // Down arrow
                    _ => selected,
                };
            }
            'w' => selected = clamp(selected - 1, 0, parts_len - 1), // 'w' for Up
            's' => selected = clamp(selected + 1, 0, parts_len - 1), // 's' for Down
            'M' => {
                if let Some(custom_partition) = get_custom_partition(termsize) {
                    selected_option = true;
                    boot::boot_from_partition(custom_partition, termsize, init_cmd);
                    return;
                }
            }
            '\n' | 'I' => {
                // Enter key or Shift+I
                selected_option = true;
                break;
            }
            'd' => {
                selected_option = true;
                init_cmd = "/bin/bash";
                break;
            }
            '\u{7f}' | 'B' => break, // Backspace or Shift+B
            '\x08' => break,         // Some systems send this as Backspace
            _ => {}
        }
    }
}
