//! Progress bars that exist purely for the audience. The real work is always
//! already done by the time they start. Off when stdout is not a terminal or
//! when `WOMM_FAST` is set, so CI stays fast and honest.

use std::fs;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

use crate::grammar::{self, Ctx};
use crate::nonet::Voice;
use crate::rng::Chooser;
use crate::state::{self, State};

pub fn enabled() -> bool {
    std::io::stdout().is_terminal() && std::env::var_os("WOMM_FAST").is_none()
}

fn bar(pct: u32) -> String {
    let filled = (pct as usize * 20) / 100;
    format!("[{}{}]", "█".repeat(filled), " ".repeat(20 - filled))
}

/// A progress bar for work that finished before it was drawn. It reaches 99%,
/// goes backwards, and lies about its ETA. It gets shorter each time you run
/// it on the same day, because it trusts you more.
pub fn lying_bar(label: &str, voice: &mut Voice) {
    if !enabled() {
        println!("   {label}: done.");
        return;
    }
    let mut st = State::load();
    if st.bar.day != state::today() {
        st.bar.day = state::today();
        st.bar.runs = 0;
    }
    st.bar.runs += 1;
    let runs = st.bar.runs;
    st.save();

    let budget = (1500 / runs.min(6)) as u64; // milliseconds, first run of the day is the full show
    let frame = |pct: u32, note: &str| {
        print!("\r\x1b[K   {label:<16} {} {pct:>3}%  {note}", bar(pct));
        let _ = std::io::stdout().flush();
    };
    let lie = (2 + voice.pick(9)).to_string();
    let steps = [(23, "ETA 3s"), (61, "ETA 2s"), (99, "ETA 1s")];
    for (pct, note) in steps {
        frame(pct, note);
        sleep(Duration::from_millis(budget / 6));
    }
    frame(97, "re-verifying");
    sleep(Duration::from_millis(budget / 4));
    frame(99, &format!("ETA {lie} business days"));
    sleep(Duration::from_millis(budget / 4));
    frame(100, &format!("done (it was done at {}%)", 5 + voice.pick(20)));
    println!();
    static SAID: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if runs > 2 && !SAID.swap(true, std::sync::atomic::Ordering::Relaxed) {
        println!("   (it was quicker this time. it trusts you now.)");
    }
}

/// The last step of `build`: a garbage collector with a trash can for a
/// progress indicator. It does really delete the intermediates.
pub fn garbage_collector(voice: &mut Voice, ctx: &Ctx, intermediates: &[PathBuf]) {
    const STEPS: usize = 6;
    const TRACK: &str = "/\\.-/\\.-/\\.-/\\.-";
    if enabled() {
        let track: Vec<char> = TRACK.chars().collect();
        for step in 0..=STEPS {
            let pos = step * (track.len() - 1) / STEPS;
            let lane: String = track
                .iter()
                .enumerate()
                .map(|(i, c)| if i == pos { "🗑".to_string() } else { c.to_string() })
                .collect();
            if step > 0 {
                let msg = grammar::say("GC", ctx, voice);
                print!("\r\x1b[K   🗑 {msg}\n");
            }
            print!("\r\x1b[K   garbage collector  {lane}");
            let _ = std::io::stdout().flush();
            if step < STEPS {
                let wait = 1000 + voice.rng().below(4001) as u64; // 1-5 seconds
                sleep(Duration::from_millis(wait));
            }
        }
        print!("\r\x1b[K");
    }
    let mut freed = 0;
    for p in intermediates {
        if p.exists() && fs::remove_dir_all(p).is_ok() {
            freed += 1;
        }
    }
    println!("   garbage collected ({freed} intermediate folder(s)).");
}
