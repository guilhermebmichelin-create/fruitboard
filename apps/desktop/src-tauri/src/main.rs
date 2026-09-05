#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if fruitboard_desktop_lib::run().is_err() {
        eprintln!("Fruitboard stopped because the native host could not continue.");
        std::process::exit(1);
    }
}
