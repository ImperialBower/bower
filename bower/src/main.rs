//! The `bower` CLI — the replay layer's front door.
//!
//! Everything the kernel refuses to do lives here: reading a book off disk,
//! parsing `bower.toml`, and writing git objects. `bower-core` stays pure, and
//! CI asserts that with `cargo tree -p bower-core -e normal`.
//!
//! As of EPIC-01 Phase 1 the configuration layer is real: both subcommands load
//! and report `bower.toml`, then exit non-zero at the point their own phase
//! takes over, so no script can mistake progress for completion.

#![warn(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod config;

use config::BookConfig;

#[derive(Debug, Parser)]
#[command(
    name = "bower",
    version,
    about = "Replay a book into deterministic git repositories."
)]
struct Cli {
    /// The book's root directory — the one holding `bower.toml`.
    #[arg(long, default_value = ".", global = true)]
    book: PathBuf,

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

    let cfg = match BookConfig::load(&cli.book) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("bower: {e}");
            return ExitCode::FAILURE;
        }
    };

    let only = match &cli.command {
        Command::Plan { repo } | Command::Build { repo, .. } => repo.as_deref(),
    };

    report(&cfg, only);

    let (name, phase) = match cli.command {
        Command::Plan { .. } => ("plan", "EPIC-01 Phase 2"),
        Command::Build { .. } => ("build", "EPIC-01 Phase 3"),
    };
    eprintln!("bower {name}: not implemented yet — see {phase}.");
    ExitCode::FAILURE
}

/// Print what the configuration says, so the parser is observable before the
/// phases that consume it exist.
fn report(cfg: &BookConfig, only: Option<&str>) {
    println!("epoch:    {}", cfg.epoch);
    println!("identity: {} <{}>", cfg.identity.name, cfg.identity.email);
    println!("site:     {}", cfg.site.as_deref().unwrap_or("(none)"));
    println!("catalog:  {} repo(s) declared to the kernel", cfg.catalog().0.len());

    for (name, repo) in &cfg.repos {
        if only.is_some_and(|want| want != name) {
            continue;
        }
        println!("\n[{name}]");
        println!("  github:              {}", opt(repo.github.as_deref()));
        println!(
            "  template:            {}",
            repo.template
                .as_deref()
                .map_or_else(|| "(none)".to_string(), |p| p.display().to_string())
        );
        println!("  verify:              {}", opt(repo.verify.as_deref()));
        println!("  keep_region_markers: {}", repo.keep_region_markers);
        println!("  links.blob:          {}", opt(repo.links.blob.as_deref()));
    }
}

fn opt(v: Option<&str>) -> &str {
    v.unwrap_or("(none)")
}
