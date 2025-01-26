use battery::Manager;
use crate::*;

pub fn show_usb_disclaimer(termsize: tui::Point) {
    // Display the USB disclaimer
    tui::move_cursor(tui::Point { row: termsize.row - 1, col: termsize.col - 41 });
    print!("Please keep the stratOS drive plugged in.");
    
    // Get battery status
    let battery_status = get_battery_status().unwrap_or("Unknown Battery".to_string());
    
    // Calculate position dynamically based on the length of the battery status text
    let col_position = termsize.col - battery_status.len() as u16;
    tui::move_cursor(tui::Point { row: termsize.row - 2, col: col_position });
    print!("{}", battery_status);
}

// Helper function to fetch the battery percentage
fn get_battery_status() -> Option<String> {
    let manager = Manager::new().ok()?;
    let battery = manager.batteries().ok()?.next()?.ok()?;
    let charge = battery.state_of_charge().value * 100.0;
    Some(format!("{:.0}% Battery", charge))
}

pub fn show_keybinds(termsize: tui::Point) {
    tui::move_cursor(tui::Point {row: termsize.row - 1, col: 2});
    print!("Use the arrow keys to move, ENTER to select, and BACKSPACE to go back.");
}

pub fn show_boot_keybinds(termsize: tui::Point) {
    tui::move_cursor(tui::Point {row: termsize.row - 1, col: 2});
    print!("Use the arrow keys to move, ENTER to boot, D to boot into a shell, and BACKSPACE to go back.");
}
