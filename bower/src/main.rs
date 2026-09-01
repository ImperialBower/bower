//! The `bower` CLI — the replay layer's front door.
//!
//! Everything the kernel refuses to do lives here: reading a book off disk,
//! parsing `bower.toml`, and writing git objects. `bower-core` stays pure, and
//! CI asserts that by checking `cargo tree -p bower-core` is empty.
//!
//! Phase 0 of EPIC-01 ships the command surface only. Both subcommands report
//! that they are unimplemented and exit non-zero, so no script can mistake a
//! skeleton for a working tool.

#![warn(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "bower",
    version,
    about = "Replay a book into deterministic git repositories."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Resolve the book into a plan, print it, and write `bower.lock`.
    Plan {
        /// Limit the run to one target repository.
        #[arg(long)]
        repo: Option<String>,
    },
    /// Replay the plan into a local git repository, one commit per step.
    Build {
        /// Limit the run to one target repository.
        #[arg(long)]
        repo: Option<String>,

        /// Directory to write the generated repository into.
        #[arg(short, long, default_value = "out")]
        out: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let (name, phase) = match cli.command {
        Command::Plan { .. } => ("plan", "EPIC-01 Phase 2"),
        Command::Build { .. } => ("build", "EPIC-01 Phase 3"),
    };

    eprintln!("bower {name}: not implemented yet — see {phase}.");
    ExitCode::FAILURE
}
