mod cli;
mod commands;
mod error;
mod model;
mod store;
mod timefmt;
mod ui;

use clap::Parser;
use owo_colors::OwoColorize;

fn main() {
    let cli = cli::Cli::parse();
    let store = store::Store::open();

    if let Err(err) = commands::dispatch(&store, cli.command) {
        eprintln!("{}", err.to_string().red());
        std::process::exit(1);
    }
}
