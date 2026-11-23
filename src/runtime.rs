//! What a womm-built executable does when run: executes its signature on the
//! Kludge VM.

use std::io::{IsTerminal, Write};

use crate::payload::Program;
use crate::vm;

pub fn run(p: Program) -> i32 {
    let code = match vm::decode_signature(&p.signature) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}: corrupt program payload: {e}", p.name);
            return 70;
        }
    };

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--womm-signature") {
        println!("{}", p.signature);
        return 0;
    }
    if args.iter().any(|a| a == "--womm-disasm") {
        print!("{}", vm::disasm(&code));
        return 0;
    }

    let stdout = std::io::stdout();
    let tty = stdout.is_terminal();
    let color = tty && std::env::var_os("NO_COLOR").is_none();
    let mut out = stdout.lock();
    let _ = writeln!(
        out,
        "{} -- built with {} from {} kludge object(s) at -O{}",
        p.name, p.built_with, p.kludge_count, p.opt_level
    );
    let _ = writeln!(out, "machine key {}, signature {}...\n", p.key_fingerprint, &p.signature[..16]);
    vm::run(&code, &mut out, tty, color);
    let _ = writeln!(out, "\nProgram complete. Works on my machine.");
    0
}
