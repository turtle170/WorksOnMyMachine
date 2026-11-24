mod cleanup;
mod gag;
mod gags;
mod grammar;
mod hidden;
mod hlo;
mod keys;
mod kludge;
mod manifest;
mod nonet;
mod payload;
mod rng;
mod runtime;
mod ship;
mod state;
mod theatre;
mod visualise;
mod vm;

use std::path::PathBuf;
use std::process::exit;

use clap::{CommandFactory, Parser, Subcommand};

/// Enterprise-grade declarative executable synthesis.
#[derive(Parser)]
#[command(
    name = "womm",
    version = concat!(env!("CARGO_PKG_VERSION"), " \"NoNet v0\""),
    about = "Enterprise-grade declarative executable synthesis.",
    long_about = "womm turns a free-form project plan (any UTF-8 text, saved as <name>.womm) \
into a native executable through a rigorous, neurally-assisted toolchain:\n\n  \
1. parse          .womm      -> .kludge intermediate objects\n  \
2. build          .kludged/  -> SHA-512 -> RSA-2048 -> hash chain (4 more signatures)\n                               \
-> NoNet extra hash -> StableHLO -> garbage collector -> .womm.md\n  \
3. parsy-builder  .womm.md   -> final hash -> Kludge VM program -> native executable\n\n\
NoNet is an 833-parameter neural network. The compiler trains it on itself before every build.\n\
Built executables accept --womm-disasm, --womm-signature, --womm-mood and --womm-no-review.",
    after_long_help = "Optimisation levels (-O0..-O3) apply to the parser, not the compiler. \
Higher levels produce more kludge objects.\n\nSee README.md for limitations.*",
    disable_help_flag = true,
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Parse a .womm project plan into .kludge intermediate objects
    Parse {
        /// Path to the <name>.womm file
        file: PathBuf,
        /// Parser optimisation level (0-3). Not for the compiler.
        #[arg(short = 'O', default_value_t = 0)]
        opt: u8,
        /// `-W no` suppresses warnings. They remain true.
        #[arg(short = 'W', value_name = "no")]
        warnings: Option<String>,
    },
    /// Compile a .kludged directory into a build manifest (.womm.md)
    Build {
        /// Path to the <name>.kludged directory produced by `womm parse`
        dir: PathBuf,
        /// `-W no` suppresses warnings. They remain true.
        #[arg(short = 'W', value_name = "no")]
        warnings: Option<String>,
    },
    /// Link a build manifest into a native executable
    #[command(name = "parsy-builder")]
    ParsyBuilder {
        /// Path to the <name>.womm.md manifest produced by `womm build`
        manifest: PathBuf,
        /// Output path for the executable (default: next to the manifest)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Visualise a build manifest
    Visualise {
        /// Path to the <name>.womm.md manifest
        manifest: PathBuf,
        /// Run the result with the bundled Python
        #[arg(long)]
        run: bool,
    },
    /// Show who wrote each line of a plan
    Blame {
        /// Path to a .womm file (or any text file)
        file: PathBuf,
    },
    /// Install a package from the package store
    Ship {
        /// Package name (you will have to find it yourself)
        name: String,
    },
}

pub fn real_help() -> String {
    Cli::command().render_long_help().to_string()
}

/// Report an error the womm way: the error, then an excuse.
pub fn fail(e: &anyhow::Error) -> i32 {
    eprintln!("womm: error: {e:#}");
    for excuse in gags::excuses(&format!("{e:#}")) {
        eprintln!("note: {excuse}");
    }
    1
}

fn quiet(w: &Option<String>) -> bool {
    w.as_deref().is_some_and(|v| v.eq_ignore_ascii_case("no"))
}

fn main() {
    // Behave like a normal Unix tool when piped into `head` or `grep -q`.
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    // A womm-built executable carries its program after the binary. If one is present,
    // this process is not womm: it is the user's project.
    if let Some(program) = payload::read_self() {
        exit(runtime::run(program));
    }

    let args: Vec<String> = std::env::args().skip(1).collect();

    // Installed packages run as their own command (e.g. `nonet` is `womm ask`).
    let argv0 = std::env::args().next().unwrap_or_default();
    let invoked_as = std::path::Path::new(&argv0)
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if invoked_as == "nonet" {
        exit(hidden::ask(&args));
    }

    if let Some(code) = gag::intercept(&args) {
        exit(code);
    }

    let cli = Cli::parse();
    let result = match cli.cmd {
        Cmd::Parse { file, opt, warnings } => kludge::parse(&file, opt, quiet(&warnings)).map(|_| 0),
        Cmd::Build { dir, warnings } => manifest::build(&dir, quiet(&warnings)).map(|_| 0),
        Cmd::ParsyBuilder { manifest, output } => manifest::link(&manifest, output).map(|_| 0),
        Cmd::Visualise { manifest, run } => visualise::visualise(&manifest, run),
        Cmd::Blame { file } => gags::blame(&file).map(|_| 0),
        Cmd::Ship { name } => ship::ship(&name),
    };
    match result {
        Ok(code) => exit(code),
        Err(e) => exit(fail(&e)),
    }
}
