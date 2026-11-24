//! What a womm-built executable does when run: notices whose machine it is on,
//! executes its final hash on the Kludge VM, then reviews its own code.

use std::io::{IsTerminal, Write};

use crate::grammar::{self, Ctx};
use crate::keys;
use crate::nonet::Voice;
use crate::payload::Program;
use crate::rng::Chooser;
use crate::vm;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mood {
    Anxious,
    Corporate,
    BurntOut,
    Smug,
    Haunted,
    Intern,
    Philosophical,
    Retired,
}

impl Mood {
    pub const ALL: [Mood; 8] = [
        Mood::Anxious,
        Mood::Corporate,
        Mood::BurntOut,
        Mood::Smug,
        Mood::Haunted,
        Mood::Intern,
        Mood::Philosophical,
        Mood::Retired,
    ];

    /// Byte 0 of the program decides. Nobody asked the program.
    pub fn of(code: &[u8]) -> Mood {
        Mood::ALL[code.first().copied().unwrap_or(0) as usize % 8]
    }

    pub fn name(self) -> &'static str {
        match self {
            Mood::Anxious => "Anxious",
            Mood::Corporate => "Corporate",
            Mood::BurntOut => "Burnt-out",
            Mood::Smug => "Smug",
            Mood::Haunted => "Haunted",
            Mood::Intern => "Intern",
            Mood::Philosophical => "Philosophical",
            Mood::Retired => "Retired",
        }
    }

    pub fn footer(self) -> &'static str {
        match self {
            Mood::Anxious => "Program complete?? Works on my machine. I think. Sorry.",
            Mood::Corporate => "Program complete. Works on my machine, per our alignment.",
            Mood::BurntOut => "program complete. works on my machine. whatever.",
            Mood::Smug => "Program complete. Works on MY machine.",
            Mood::Haunted => "Program complete. It worked on a machine. Not this one.",
            Mood::Intern => "program complete!! works on my machine (i think?? first day)",
            Mood::Philosophical => "Program complete. Works on my machine. What is a machine, really.",
            Mood::Retired => "Program complete. Works on my machine. I used to work on machines.",
        }
    }

    fn reviewer(self) -> &'static str {
        match self {
            Mood::Anxious => "self (sorry in advance)",
            Mood::Corporate => "self (on behalf of the review committee)",
            Mood::BurntOut => "self (skimmed it)",
            Mood::Smug => "self (the best reviewer available)",
            Mood::Haunted => "self, and something else",
            Mood::Intern => "self (first review ever!!)",
            Mood::Philosophical => "self (can code review itself?)",
            Mood::Retired => "self (back from retirement for this)",
        }
    }
}

fn foreign_hostname(hash: &str) -> String {
    const BASES: [&str; 8] = [
        "MacBook-Pro", "DESKTOP", "build-server", "Kevins-Laptop", "prod-01", "ubuntu-latest",
        "the-good-mac", "WORKSTATION",
    ];
    let b = hex::decode(hash).unwrap_or_default();
    let base = BASES[b.first().copied().unwrap_or(0) as usize % BASES.len()];
    format!("{base}-{}", hash.get(..4).unwrap_or("0000").to_uppercase())
}

fn translated() -> bool {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("sysctl")
            .args(["-n", "sysctl.proc_translated"])
            .output()
            .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).trim() == "1")
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Complain (then run anyway) when this is not the machine it was built on.
fn machine_lock(p: &Program, mood: Mood, voice: &mut Voice, out: &mut dyn Write) {
    let here_key = keys::existing_key().map(|k| keys::fingerprint(&k));
    let here_host = keys::host_hash();
    let key_differs = here_key.as_deref() != Some(p.key_fingerprint.as_str());
    let host_differs = !p.host_hash.is_empty() && p.host_hash != here_host;
    if !key_differs && !host_differs {
        if translated() {
            let _ = writeln!(out, "also, you are emulating me. rude.\n");
        }
        return;
    }
    let name = gethostname::gethostname().to_string_lossy().into_owned();
    let _ = writeln!(out, "⚠ machine mismatch");
    let _ = writeln!(
        out,
        "  - expected: {} ({}, key {})",
        foreign_hostname(&p.host_hash),
        p.arch,
        p.key_fingerprint
    );
    let _ = writeln!(
        out,
        "  + found:    {name} ({}, key {})",
        std::env::consts::ARCH,
        here_key.unwrap_or_else(|| "????????????????".into())
    );
    let _ = writeln!(out, "this was built on my machine. you are not my machine.");
    let protest = grammar::say("PROTEST", &Ctx::default(), voice);
    let protest = if mood == Mood::Smug { format!("{protest}, obviously") } else { protest };
    let _ = writeln!(out, "running anyway. ({protest})");
    if translated() {
        let _ = writeln!(out, "also, you are emulating me. rude.");
    }
    let _ = writeln!(out);
}

/// The program reviews its own instructions. The facts are real (see
/// --womm-disasm); NoNet only supplies the attitude.
pub fn review(code: &[u8], mood: Mood, voice: &mut Voice) -> String {
    let a = vm::analyze(code);
    let ctx = Ctx::default();
    let mut notes: Vec<(usize, String)> = Vec::new();

    let first = |op: u8| code.iter().position(|b| b >> 4 == op && *b != 0xFF);
    if a.stray_ends > 0 {
        let pc = stray_end_pc(code).unwrap_or(0);
        notes.push((pc, format!("{} stray END(s); who are these ENDs for", a.stray_ends)));
    }
    let excuses = a.histogram[0xD];
    if excuses >= 4 {
        let pc = first(0xD).unwrap_or(0);
        notes.push((pc, format!("EXCUSE appears {excuses} times, which is {excuses} more than the spec")));
    }
    if a.max_depth >= 3 {
        let pc = first(0x5).unwrap_or(0);
        notes.push((pc, format!("loop depth {} reached", a.max_depth)));
    }
    if a.unterminated_loops > 0 {
        notes.push((code.len() - 1, format!("{} loop(s) never END", a.unterminated_loops)));
    }
    match a.halt_at {
        Some(pc) => notes.push((pc, format!("HALT here; {} instruction(s) after it never run", code.len() - pc - 1))),
        None => notes.push((code.len() - 1, "no HALT. it just... stops".to_string())),
    }
    let (top, count) = a.histogram.iter().enumerate().max_by_key(|(_, n)| **n).unwrap();
    notes.push((
        first(top as u8).unwrap_or(0),
        format!("{} x{count} is a lot of {}", vm::MNEMONICS[top], vm::MNEMONICS[top]),
    ));

    let mut s = String::from("── review ────────────────────────────────────────\n");
    s.push_str(&format!("reviewer: {} (conflict of interest: yes)\n", mood.reviewer()));
    for (pc, fact) in notes.into_iter().take(5) {
        let opener = grammar::say("REVIEW_OPENER", &ctx, voice);
        let tail = grammar::say("REVIEW_TAIL", &ctx, voice);
        s.push_str(&format!("  {pc:#06x}  {opener} {fact}; {tail}\n"));
    }
    let score = a.stray_ends * 2 + a.unterminated_loops * 3 + excuses + usize::from(a.halt_at.is_none());
    let verdict = if voice.one_in(32) {
        "LGTM"
    } else if score > 30 {
        "REQUEST CHANGES (will merge anyway)"
    } else if score > 12 {
        "APPROVED WITH RESERVATIONS"
    } else {
        "COMMENT (no decision, as is tradition)"
    };
    s.push_str(&format!("verdict: {verdict}\n"));
    s
}

fn stray_end_pc(code: &[u8]) -> Option<usize> {
    let mut depth = 0usize;
    for (pc, b) in code.iter().enumerate() {
        match b >> 4 {
            0x5 => depth += 1,
            0x6 if depth > 0 => depth -= 1,
            0x6 => return Some(pc),
            _ => {}
        }
    }
    None
}

pub fn run(p: Program) -> i32 {
    let code = match vm::decode_program(&p.signature) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}: corrupt program payload: {e}", p.name);
            return 70;
        }
    };
    let mood = Mood::of(&code);

    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |f: &str| args.iter().any(|a| a == f);
    if flag("--womm-signature") {
        println!("{}", p.signature);
        return 0;
    }
    if flag("--womm-disasm") {
        print!("{}", vm::disasm(&code));
        return 0;
    }
    let mut voice = Voice::new(&code);
    if flag("--womm-mood") {
        println!("{}", mood.name());
        println!("horoscope: {}", grammar::say("HOROSCOPE", &Ctx::default(), &mut voice));
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
    let _ = writeln!(
        out,
        "machine key {}, {} instructions, mood: {}\n",
        p.key_fingerprint,
        code.len(),
        mood.name()
    );
    machine_lock(&p, mood, &mut voice, &mut out);
    vm::run(&code, &mut out, tty && std::env::var_os("WOMM_FAST").is_none(), color);
    let _ = writeln!(out, "\n{}", mood.footer());
    if flag("--womm-no-review") {
        let _ = writeln!(out, "review skipped. the code knows what it did.");
    } else {
        let _ = write!(out, "\n{}", review(&code, mood, &mut voice));
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nonet::Nonet;

    #[test]
    fn review_cites_real_program_counters() {
        let mut code = vec![0x00u8; 300];
        code[10] = 0x61; // stray END
        code[200] = 0xFF; // HALT
        let mut voice = Voice::with_net(Nonet::untrained(b"r"), b"r");
        let r = review(&code, Mood::Smug, &mut voice);
        assert!(r.contains("0x000a"), "{r}");
        assert!(r.contains("0x00c8"), "{r}");
        assert!(r.contains("verdict:"));
    }

    #[test]
    fn every_mood_has_a_footer() {
        for m in Mood::ALL {
            assert!(m.footer().to_lowercase().contains("program complete"));
        }
    }
}
