// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args_os().any(|arg| arg == "--silent") {
        std::process::exit(rawaccel_userspace_lib::startup::run_silent());
    }

    rawaccel_userspace_lib::run()
}
