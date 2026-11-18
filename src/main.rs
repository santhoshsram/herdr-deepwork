//! `herdr-focus`: a focus timer for the herdr tab bar.

use std::process::ExitCode;

fn main() -> ExitCode {
    let command = std::env::args().nth(1);
    match command.as_deref() {
        Some("status") => {
            println!("{}", herdr_focus::status_line());
            ExitCode::SUCCESS
        }
        Some("panel") => herdr_focus::panel::run(),
        _ => {
            eprintln!("usage: herdr-focus status | panel");
            ExitCode::from(2)
        }
    }
}
