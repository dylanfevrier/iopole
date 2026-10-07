//! Point d'entrée du binaire `iopole`.

use clap::Parser;
use iopole::Cli;

fn main() {
    Cli::parse();
}
