//! Commands that appear in no help output: ask, train, standup, why, the
//! works-on-my-machine certificate, duel, and feeling better.

use std::fs;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256, Sha512};

use crate::grammar::{self, Ctx};
use crate::keys;
use crate::nonet::{self, Nonet, Voice};
use crate::payload;
use crate::rng::{Chooser, Rng};
use crate::runtime::Mood;
use crate::ship;
use crate::state::{self, State};
use crate::theatre;
use crate::vm;

const ENDGAME: u32 = 100;

/// 1 in this many questions gets a yes. Training makes it rarer.
fn yes_odds(epochs: u32) -> u64 {
    let t = epochs.min(ENDGAME - 1) as f64 / (ENDGAME - 1) as f64;
    (500.0 * 100f64.powf(t)) as u64
}

fn no_rate(epochs: u32) -> f64 {
    (91.2 + 8.8 * epochs.min(ENDGAME) as f64 / ENDGAME as f64).min(100.0)
}

fn pause(ms: u64) {
    if theatre::enabled() {
        sleep(Duration::from_millis(ms));
    }
}

pub fn ask(words: &[String]) -> i32 {
    if !ship::installed("nonet") {
        println!("No.");
        return 0;
    }
    let mut st = State::load();
    st.nonet.asks += 1;
    st.day(state::today()).asks += 1;
    let epochs = st.nonet.epochs;
    st.save();

    if epochs >= ENDGAME {
        println!("no.");
        return 0;
    }
    let question = words.join(" ").to_lowercase();
    let question = question.trim().trim_end_matches('?').trim();
    if question.is_empty() {
        println!("No.");
        return 0;
    }

    let mut voice = Voice::new(&[b"ask:".as_slice(), question.as_bytes()].concat());
    let mut ctx = Ctx::from_text(question);
    ctx.topic = ctx.words.first().cloned().unwrap_or_else(|| question.to_string());

    println!("NoNet v0 · {} parameters · thinking...", nonet::PARAMS);
    for _ in 0..3 {
        pause(350);
        println!("  ▸ {}", grammar::say("ASK_STEP", &ctx, &mut voice));
    }
    pause(500);

    let h = u64::from_le_bytes(Sha256::digest(question.as_bytes())[..8].try_into().unwrap());
    let confidence = 90.0 + ((voice.opinion().abs() * 1000.0) as u32 % 100) as f32 / 10.0;
    if h % yes_odds(epochs) == 0 {
        println!("{:<40}(confidence {:.1}%)", "Yes.", 100.0 - confidence + 0.1);
        println!("  (NoNet has never said that before. write this question down.)");
        return 0;
    }

    let mut unlocked = vec![];
    if epochs >= 25 {
        unlocked.push("Computer says no.");
    }
    if epochs >= 50 {
        unlocked.push("No (with feeling).");
    }
    if epochs >= 75 {
        unlocked.push("The answer was no before you asked.");
    }
    let answer = if !unlocked.is_empty() && voice.one_in(3) {
        unlocked[voice.pick(unlocked.len())].to_string()
    } else {
        grammar::say("NO", &ctx, &mut voice)
    };
    println!("{answer:<40}(confidence {confidence:.1}%)");
    if voice.one_in(2) {
        println!("  reasoning: {}.", grammar::say("NO_REASON", &ctx, &mut voice));
    }
    0
}

pub fn train(args: &[String]) -> i32 {
    if !ship::installed("nonet") {
        println!("No.");
        return 0;
    }
    if args.iter().any(|a| a == "--unlearn") {
        println!("cannot unlearn. that's not how this works. (it is, but no.)");
        return 0;
    }
    let epochs: u32 = args
        .iter()
        .position(|a| a == "--epochs")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(5)
        .clamp(1, 50);

    let mut st = State::load();
    let start = st.nonet.epochs;
    if start >= ENDGAME {
        println!("training complete. there is nothing left to learn. the answer is no.");
        return 0;
    }
    let key = keys::existing_key().map(|k| keys::public_der(&k)).unwrap_or_default();
    let mut net = Nonet::untrained(&key);
    let binary = payload::base_stub().unwrap_or_default();

    for e in 1..=epochs {
        let global = start + e;
        net.train(&binary, &[key.as_slice(), &global.to_le_bytes()].concat(), 20);
        let loss = net.eval(&binary);
        let note = match global {
            ENDGAME => "   (it's learned)",
            g if g.is_multiple_of(25) => "   (it's learning)",
            _ => "",
        };
        println!(
            "epoch {e}/{epochs}  loss {loss:.4}  no-rate {:.1}%{note}",
            no_rate(global)
        );
        for (tier, unlock) in [(25, "Computer says no."), (50, "No (with feeling)."), (75, "The answer was no before you asked.")] {
            if global == tier {
                println!("  unlocked: `{unlock}`");
            }
        }
        pause(120);
        if global >= ENDGAME {
            break;
        }
    }
    let trained = (start + epochs).min(ENDGAME);
    st.nonet.epochs = trained;
    st.day(state::today()).epochs += trained - start;
    st.save();
    if trained >= ENDGAME {
        println!("model saved. it no longer needs to hear the question.");
    } else {
        println!("model saved. it has become more itself.");
    }
    0
}

pub fn standup() -> i32 {
    let st = State::load();
    let y = st.day_ref(state::today().saturating_sub(1));
    let t = st.day_ref(state::today());
    let mut voice = Voice::new(&[b"standup:".as_slice(), &state::today().to_le_bytes()].concat());
    let favourite = st
        .cleanup
        .by_phrase
        .iter()
        .max_by_key(|(_, n)| **n)
        .map(|(p, _)| p.clone())
        .unwrap_or_else(|| "augh".into());

    let describe = |d: &state::Day| {
        let mut parts = Vec::new();
        if d.breakdowns > 0 {
            parts.push(format!("said \"{favourite}\" {} time(s)", d.breakdowns));
        }
        if d.builds > 0 {
            let mut moods = d.moods.clone();
            moods.sort();
            moods.dedup();
            parts.push(format!("built {} executable(s) ({})", d.builds, moods.join(", ").to_lowercase()));
        }
        if d.asks > 0 {
            parts.push(format!("asked NoNet {} question(s), got {} no(s)", d.asks, d.asks));
        }
        if d.epochs > 0 {
            parts.push(format!("trained NoNet for {} epoch(s)", d.epochs));
        }
        if parts.is_empty() {
            "nothing. impressive restraint.".to_string()
        } else {
            parts.join(", ") + "."
        }
    };
    println!("yesterday: {}", describe(&y));
    println!("so far today: {}", describe(&t));
    let plan = if y.breakdowns + t.breakdowns > 0 {
        format!("will say \"{favourite}\"")
    } else if y.builds + t.builds > 0 {
        "will build it again and hope".into()
    } else {
        "will open the laptop. that's the plan.".into()
    };
    println!("today: {plan}.");
    println!("blockers: {}.", grammar::say("BLOCKER", &Ctx::default(), &mut voice));
    0
}

pub fn why(release: bool) -> i32 {
    let seed = if release { env!("CARGO_PKG_VERSION").as_bytes().to_vec() } else { state::now().to_le_bytes().to_vec() };
    let mut voice = Voice::new(&[b"why:".as_slice(), &seed].concat());
    let ctx = Ctx::default();
    if release {
        println!("Enterprise-grade declarative executable synthesis. Works on my machine.\n");
        println!("## Design rationale (generated by NoNet)\n");
        for _ in 0..5 {
            println!("- {}.", grammar::say("WHY", &ctx, &mut voice));
        }
    } else {
        println!("{}.", grammar::say("WHY", &ctx, &mut voice));
    }
    0
}

const CERT_BEGIN: &str = "-----BEGIN WOMM SIGNATURE-----";
const CERT_END: &str = "-----END WOMM SIGNATURE-----";

fn cert_message(host: &str, day: &str, version: &str) -> Vec<u8> {
    Sha512::digest(format!("it works on my machine|{host}|{day}|{version}")).to_vec()
}

pub fn certificate(args: &[String]) -> Result<i32> {
    if let Some(i) = args.iter().position(|a| a == "--verify") {
        let path = args.get(i + 1).context("--verify needs a certificate file")?;
        return verify_certificate(Path::new(path));
    }
    if !args.iter().any(|a| a == "--prove") {
        println!("it does. (try --prove)");
        return Ok(0);
    }
    let key = keys::machine_key()?;
    let host = keys::host_hash();
    let day = state::today().to_string();
    let version = env!("CARGO_PKG_VERSION");
    let sig = keys::sign(&key, &cert_message(&host, &day, version))?;
    println!("╔══════════════════════════════════════════════════╗");
    println!("║     CERTIFICATE OF WORKS-ON-MY-MACHINE           ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!("claim: it works on my machine");
    println!("machine: {host}");
    println!("key: {}", keys::fingerprint(&key));
    println!("day: {day}");
    println!("womm: {version}");
    println!("{CERT_BEGIN}");
    for line in hex::encode(sig).as_bytes().chunks(64) {
        println!("{}", std::str::from_utf8(line).unwrap());
    }
    println!("{CERT_END}");
    print!("{}", keys::public_pem(&key)?);
    println!("(this certificate is cryptographically valid and legally meaningless)");
    Ok(0)
}

fn verify_certificate(path: &Path) -> Result<i32> {
    let text = fs::read_to_string(path).with_context(|| format!("could not read {}", path.display()))?;
    let field = |k: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(&format!("{k}: ")).map(str::to_string))
            .with_context(|| format!("certificate has no {k}"))
    };
    let (host, day, version) = (field("machine")?, field("day")?, field("womm")?);
    let start = text.find(CERT_BEGIN).context("certificate has no signature")? + CERT_BEGIN.len();
    let end = text.find(CERT_END).context("certificate signature is unterminated")?;
    let sig = hex::decode(text[start..end].split_whitespace().collect::<String>())?;
    let pem_start = text.find("-----BEGIN PUBLIC KEY-----").context("certificate has no public key")?;
    let pem_end = text.find("-----END PUBLIC KEY-----").context("public key is unterminated")?
        + "-----END PUBLIC KEY-----".len();
    let pem = format!("{}\n", &text[pem_start..pem_end]);
    match keys::verify(&pem, &cert_message(&host, &day, &version), &sig) {
        Ok(()) => {
            println!("VALID. it works on that machine. (not yours.)");
            Ok(0)
        }
        Err(_) => {
            println!("INVALID. it does not work on any machine.");
            Ok(1)
        }
    }
}

struct Fighter {
    name: String,
    mood: Mood,
    hp: i32,
    attack: i32,
    defence: i32,
}

fn fighter(path: &Path) -> Result<(Fighter, Vec<u8>)> {
    let p = payload::read_from(path).with_context(|| format!("{} is not a womm executable", path.display()))?;
    let code = vm::decode_program(&p.signature)?;
    let a = vm::analyze(&code);
    Ok((
        Fighter {
            name: p.name,
            mood: Mood::of(&code),
            hp: 40 + (code.len() % 40) as i32,
            attack: 2 + a.histogram[0xD] as i32 / 8,
            defence: a.max_depth as i32,
        },
        code,
    ))
}

/// Two executables fight. The result depends only on the two programs, so it
/// is the same on every machine.
pub fn duel(args: &[String]) -> Result<i32> {
    let [a, b] = args else { bail!("usage: womm duel <executable> <executable>") };
    let (mut fa, ca) = fighter(Path::new(a))?;
    let (mut fb, cb) = fighter(Path::new(b))?;
    if ca == cb {
        println!("you cannot duel yourself. (you can. it's a draw. it's always a draw.)");
        return Ok(0);
    }
    // Order the fighters by program so `a vs b` and `b vs a` are the same fight.
    let (lo, hi) = if ca < cb { (&ca, &cb) } else { (&cb, &ca) };
    if ca > cb {
        std::mem::swap(&mut fa, &mut fb);
    }
    let mut rng = Rng::from_seed(&[b"duel:".as_slice(), lo, hi].concat());
    let ctx = Ctx::default();
    println!("{} ({}, {} hp) vs {} ({}, {} hp)\n", fa.name, fa.mood.name(), fa.hp, fb.name, fb.mood.name(), fb.hp);

    for f in [&fa, &fb] {
        if f.mood == Mood::Retired {
            println!("{} is retired. it refuses to fight. it has seen too much.", f.name);
            println!("\nresult: draw. nobody works on anybody's machine.");
            return Ok(0);
        }
    }
    let mut fighters = [fa, fb];
    for turn in 0..30 {
        let (atk, def) = if turn % 2 == 0 { (0, 1) } else { (1, 0) };
        if turn < 2 && fighters[atk].mood == Mood::Intern {
            println!("{} forfeits on turn 1. it was its first day.", fighters[atk].name);
            return finish(&fighters[def]);
        }
        let mut dmg = (fighters[atk].attack + rng.below(6) as i32 - fighters[def].defence).max(1);
        if fighters[atk].mood == Mood::Smug && fighters[def].mood == Mood::Anxious {
            dmg += 2;
        }
        let excuse = grammar::say("EXCUSE", &ctx, &mut rng);
        if fighters[def].mood == Mood::Haunted && turn % 4 == 1 {
            println!("{} hits {}, but the hit goes straight through. it is haunted.", fighters[atk].name, fighters[def].name);
            continue;
        }
        fighters[def].hp -= dmg;
        println!(
            "{} throws {dmg} EXCUSE(s) at {}. {}: \"{excuse}\" ({} hp)",
            fighters[atk].name,
            fighters[def].name,
            fighters[def].name,
            fighters[def].hp.max(0)
        );
        if fighters[def].hp <= 0 {
            return finish(&fighters[atk]);
        }
    }
    println!("\nresult: draw. both still work on their own machines.");
    Ok(0)
}

fn finish(winner: &Fighter) -> Result<i32> {
    println!("\nwinner: {} ({})", winner.name, winner.mood.name());
    println!("{}: works on MY machine.", winner.name);
    Ok(0)
}

pub fn feel_better() -> i32 {
    let mut st = State::load();
    st.cleanup.total = 0;
    st.cleanup.by_phrase.clear();
    st.save();
    println!("that is the first lie you've told me.");
    0
}

/// What a breakdown is called, for the stats.
pub fn phrase_key(norm: &str) -> String {
    let w = norm.replace(' ', "");
    if w.starts_with('a') && w.ends_with('h') && w.contains('u') {
        "augh".into()
    } else if w.starts_with('n') && w.chars().skip(1).all(|c| c == 'o') {
        "nooo".into()
    } else {
        norm.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn training_only_makes_yes_rarer() {
        assert_eq!(yes_odds(0), 500);
        assert!(yes_odds(50) > yes_odds(10));
        assert_eq!(yes_odds(99), 50_000);
        assert!(no_rate(100) >= 100.0);
    }

    #[test]
    fn phrase_keys_collapse_variants() {
        assert_eq!(phrase_key("auuughhh"), "augh");
        assert_eq!(phrase_key("noooo"), "nooo");
        assert_eq!(phrase_key("fart"), "fart");
    }
}
