use std::io;
use std::process::ExitCode;

use clap::Parser;
use iopole::Cli;

fn main() -> ExitCode {
    let cli = Cli::parse();
    // Not locked for the whole run: `iopole mcp` hands stdout to its transport.
    let mut stdout = io::stdout();

    match iopole::run(&cli, &mut stdout) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}
