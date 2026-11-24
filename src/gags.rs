//! The toolchain's opinions: generated warnings, -O passes, excuses, kludge
//! rot and `womm blame`. All of it is spoken by NoNet through the grammar.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::grammar::{self, Ctx};
use crate::keys;
use crate::nonet::Voice;
use crate::rng::Chooser;
use crate::state;

pub struct Warning {
    pub code: String,
    pub line: usize,
    pub text: String,
}

/// Warnings about a plan. Each one cites a real line and quotes words from it.
pub fn warnings(voice: &mut Voice, source: &str, opt: u8) -> Vec<Warning> {
    let lines: Vec<(usize, &str)> = source
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| (i + 1, l))
        .collect();
    if lines.is_empty() {
        return Vec::new();
    }
    let whole = Ctx::from_text(source);
    let count = 1 + voice.pick(4) + opt as usize;
    (0..count)
        .map(|_| {
            let (line, text) = lines[voice.pick(lines.len())];
            let mut ctx = Ctx::from_text(text);
            if ctx.words.is_empty() {
                ctx.words = whole.words.clone();
            }
            Warning {
                code: format!("{:04x}", voice.pick(0x10000)),
                line,
                text: grammar::say("WARN_BODY", &ctx, voice),
            }
        })
        .collect()
}

pub fn print_warnings(ws: &[Warning], earlier: Option<&[Warning]>, suppressed: bool) {
    if suppressed {
        println!("   warnings suppressed. they are still true.");
        return;
    }
    for w in ws {
        let still = earlier.is_some_and(|e| e.iter().any(|p| p.line == w.line));
        let still = if still { " (still)" } else { "" };
        println!("warning[W{}]: line {}: {}{still}", w.code, w.line, w.text);
    }
    if !ws.is_empty() {
        println!("womm: {} warning(s) emitted, 0 heeded", ws.len());
    }
}

/// The seed for everything said at parse time, before any signature exists.
pub fn parse_seed(key_der: &[u8], source_sha256: &str) -> Vec<u8> {
    [b"parse:".as_slice(), key_der, source_sha256.as_bytes()].concat()
}

/// Fake optimisation passes, one per -O level, plus -O3's concerns.
pub fn opt_passes(voice: &mut Voice, ctx: &Ctx, opt: u8) {
    for level in 1..=opt {
        let pass = grammar::say("PASS", ctx, voice);
        let result = grammar::say("PASS_RESULT", ctx, voice);
        let dots = ".".repeat(36usize.saturating_sub(pass.chars().count()).max(3));
        println!("  -O{level}  pass {level}/{opt}  {pass} {dots} {result}");
    }
    if opt == 3 {
        println!("note: this is a lot of kludge. are you okay?");
    }
}

pub fn o3_aftermath(total: u32, opt: u8) {
    let mut st = state::State::load();
    st.o3_streak = if opt == 3 { st.o3_streak + 1 } else { 0 };
    let streak = st.o3_streak;
    st.save();
    if opt == 3 {
        println!("note: {total} kludge objects. that is {total} more than anyone needs.");
        if streak >= 3 {
            println!("note: you keep doing this.");
        }
    }
}

/// Excuses for a real error. Stable per error message and machine.
pub fn excuses(message: &str) -> Vec<String> {
    let fp = keys::existing_key().map(|k| keys::fingerprint(&k)).unwrap_or_default();
    let mut voice = Voice::new(&[b"excuse:".as_slice(), message.as_bytes(), fp.as_bytes()].concat());
    let ctx = Ctx::default();
    let mut out = vec![grammar::say("EXCUSE", &ctx, &mut voice)];
    if voice.one_in(64) {
        out.push(format!("disregard that. {}", grammar::say("EXCUSE", &ctx, &mut voice)));
    }
    out
}

/// How long each kludge object has been sitting there.
pub fn kludge_rot(dir: &Path, voice: &mut Voice, ctx: &Ctx) {
    let now = std::time::SystemTime::now();
    let mut aged: Vec<(u64, String)> = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "kludge") {
                let days = e
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| now.duration_since(t).ok())
                    .map(|d| d.as_secs() / 86_400)
                    .unwrap_or(0);
                let idx = p.file_name().unwrap().to_string_lossy().chars().take(4).collect::<String>();
                aged.push((days, idx.trim_start_matches('0').to_string()));
            }
        }
    }
    aged.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for (days, idx) in aged.into_iter().filter(|(d, _)| *d >= 1).take(3) {
        let idx = if idx.is_empty() { "0".into() } else { idx };
        let (tier, what) = match days {
            30.. => ("ROT_ANCIENT", "is ancient"),
            7.. => ("ROT_OLD", "has gone off"),
            _ => ("ROT_STALE", "has gone stale"),
        };
        let says = grammar::say(tier, ctx, voice);
        println!("warning[rot]: kludge #{idx} {what} ({days} day(s)). it {says}.");
    }
}

/// Hand-written, because some things should come from the heart.
const BLAME: &[&str] = &[
    "why didn't you remember",
    "seriously",
    "you know what you did",
    "this one has your fingerprints on it. literally.",
    "git blame says you. i say you. we agree.",
    "you had one job and it was this line",
    "this was fine until you touched it",
    "you said \"quick fix\" in a tone i didn't trust",
    "who approved this? you. you approved this.",
    "you, but in a hurry",
    "i'm not mad. i'm just disappointed.",
    "this line remembers you even if you don't remember it",
    "you copy-pasted this from a 2014 forum post",
    "you could have written a test. you wrote a vibe.",
    "it's always you",
    "past you did this. present you is also implicated.",
    "you named a file final_final_2 and i haven't forgotten",
    "you said \"it's temporary\" three years ago",
    "not the machine's fault. the machine is innocent.",
    "even the linter gave up on this one",
    "stakeholders asked for it, but you said yes",
    "you, again",
    "you wrote this with the confidence of someone who never ran it",
    "this is load-bearing and it's your fault",
    "the comment says TODO. the author says you.",
    "you explained this in standup and nobody understood",
    "you",
    "bold of you",
    "you thought no one would read this far",
    "this line has a sequel and you wrote that too",
    "\"works on my machine\" -- you, about this line",
    "you could have closed the laptop. you didn't.",
];
const SUFFIX: &[&str] = &["", "", "", " (again)", " (at 2am)", " (on a Friday)", " (in staging)"];
const WHEN: &[&str] = &[
    "2 minutes ago", "an hour ago", "this morning", "yesterday", "last night, 2am", "last week",
    "last sprint", "before the reorg", "3 years ago", "in a previous life",
];

pub fn blame(file: &Path) -> Result<()> {
    let text = fs::read_to_string(file).with_context(|| format!("could not read {}", file.display()))?;
    let key = keys::machine_key()?;
    let fp = keys::fingerprint(&key);
    let mut voice = Voice::new(&[b"blame:".as_slice(), &keys::public_der(&key), text.as_bytes()].concat());
    let lines: Vec<&str> = text.lines().collect();
    let total = lines.len().max(1);
    let width = total.to_string().len();
    for (i, line) in lines.iter().enumerate() {
        let when = WHEN[((i * WHEN.len()) / total + voice.pick(2)).min(WHEN.len() - 1)];
        let comment = if line.trim().is_empty() {
            "you left this blank. even this is on you.".to_string()
        } else {
            format!("{}{}", BLAME[voice.pick(BLAME.len())], SUFFIX[voice.pick(SUFFIX.len())])
        };
        println!("{} (you, {when:<18}) {:>width$} | {line}    -- {comment}", &fp[..8], i + 1);
    }
    println!("\n100.0% of lines authored by you. 0.0% by the machine. the machine is innocent.");
    Ok(())
}
