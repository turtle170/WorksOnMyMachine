mod cleanup;
mod gag;
mod keys;
mod kludge;
mod manifest;
mod payload;
mod runtime;
mod vm;

use std::path::PathBuf;
use std::process::exit;

use clap::{CommandFactory, Parser, Subcommand};

/// Enterprise-grade declarative executable synthesis.
#[derive(Parser)]
#[command(
    name = "womm",
    version = concat!(env!("CARGO_PKG_VERSION"), " (works-on-my-machine)"),
    about = "Enterprise-grade declarative executable synthesis.",
    long_about = "womm turns a free-form project plan (any UTF-8 text, saved as <name>.womm) \
into a native executable through a rigorous three-stage toolchain:\n\n  \
1. parse          .womm        -> .kludge intermediate objects\n  \
2. build          .kludged/    -> SHA-512 -> RSA-2048 signature -> .womm.md manifest\n  \
3. parsy-builder  .womm.md     -> 512 hex digits -> Kludge VM program -> native executable\n\n\
Built executables accept --womm-disasm and --womm-signature.",
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
        #[arg(short = 'O', default_value_t = 0, value_parser = clap::value_parser!(u8).range(0..=3))]
        opt: u8,
    },
    /// Compile a .kludged directory into a build manifest (.womm.md)
    Build {
        /// Path to the <name>.kludged directory produced by `womm parse`
        dir: PathBuf,
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
}

pub fn real_help() -> String {
    Cli::command().render_long_help().to_string()
}

fn main() {
    // A womm-built executable carries its program after the binary. If one is present,
    // this process is not womm: it is the user's project.
    if let Some(program) = payload::read_self() {
        exit(runtime::run(program));
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = gag::intercept(&args) {
        exit(code);
    }

    let cli = Cli::parse();
    let result = match cli.cmd {
        Cmd::Parse { file, opt } => kludge::parse(&file, opt),
        Cmd::Build { dir } => manifest::build(&dir),
        Cmd::ParsyBuilder { manifest, output } => manifest::link(&manifest, output),
    };
    if let Err(e) = result {
        eprintln!("womm: error: {e:#}");
        eprintln!("note: this worked on my machine");
        exit(1);
    }
}
