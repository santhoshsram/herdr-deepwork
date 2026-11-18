//! `herdr-deepwork`: a focus timer for the herdr tab bar.

use std::process::ExitCode;

fn main() -> ExitCode {
    let command = std::env::args().nth(1);
    match command.as_deref() {
        Some("status") => {
            println!("{}", herdr_deepwork::status_line());
            ExitCode::SUCCESS
        }
        Some("panel") => herdr_deepwork::panel::run(),
        _ => {
            eprintln!("usage: herdr-deepwork status | panel");
            ExitCode::from(2)
        }
    }
}
