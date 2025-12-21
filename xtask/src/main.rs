use crate::commands::Core;
use clap::Parser;

mod commands;

#[derive(Parser, Debug, Clone)]
enum Args {
    /// Builds the `app_core` and `net_core` elfs, links them together, flashes them onto the
    /// device using probe-rs, and listens.
    Run {
        /// Needs to be here, as when called as a runner cargo automatically calls it with the
        /// binary, which we don't care about
        _elf_path: Option<String>,
    },
    /// Runs a debugger using the previously-built binary
    Debug { core: Core },
}

fn main() {
    tracing_subscriber::fmt().init();

    let args = Args::parse();
    let result = match args {
        Args::Run { .. } => commands::run(),
        Args::Debug { core } => commands::debug(core),
    };
    if let Err(error) = result {
        println!("{error:?}");
    };
}
