#![windows_subsystem = "windows"]

use std::process::ExitCode;

use common::{os::App, util::print_debug_string};

fn main() -> ExitCode {
    let message = "Hello, D3D12!".to_string();
    print_debug_string(&message);

    let (mut app, window) = match App::init("Hello Window", (800, 600)) {
        Ok((app, window)) => (app, window),
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    while app.run() {
        let _ = window;
    }

    ExitCode::SUCCESS
}
