//! The one useful feature: remove every .kludge object, empty .kludged
//! directory and intermediate .womm.md manifest under the current directory.
//! Source .womm files and built executables are never touched.

use std::fs;
use std::path::Path;

use crate::hidden::phrase_key;
use crate::manifest::MAGIC_PREFIX;
use crate::state::{self, State};

#[derive(Default)]
struct Report {
    kludges: usize,
    dirs: usize,
    manifests: usize,
}

fn ago(secs: u64) -> String {
    match secs {
        0..=89 => "just now".into(),
        90..=5399 => format!("{} minutes ago", secs / 60),
        5400..=129_599 => format!("{} hours ago", secs / 3600),
        _ => format!("{} days ago", secs / 86_400),
    }
}

/// The cleanup commands remember you.
fn remember(phrase: &str) {
    let mut st = State::load();
    let last = st.cleanup.last;
    st.cleanup.total += 1;
    st.cleanup.last = state::now();
    *st.cleanup.by_phrase.entry(phrase_key(phrase)).or_insert(0) += 1;
    st.day(state::today()).breakdowns += 1;
    let total = st.cleanup.total;
    let favourite = st.cleanup.by_phrase.iter().max_by_key(|(_, n)| **n).map(|(p, n)| (p.clone(), *n));
    st.save();

    let tier = match total {
        100.. if total.is_multiple_of(100) => Some("CERTIFICATE OF PERSEVERANCE: awarded for 100 breakdowns. frame it."),
        50.. => Some("at this point i am the one who wants to stop."),
        25.. => Some("that is a personal best."),
        10.. => Some("have you considered touching grass. it compiles on every machine."),
        5.. => Some("this is becoming a pattern."),
        3.. => Some("third time. no judgement. some judgement."),
        _ => None,
    };
    if let Some(t) = tier {
        println!("(breakdown #{total}) {t}");
    }
    if total >= 3 {
        if let Some((p, n)) = favourite {
            if n >= 2 {
                println!("you mostly say \"{p}\" when stressed.");
            }
        }
    }
    if last > 0 {
        println!("last breakdown: {}.", ago(state::now().saturating_sub(last)));
    }
}

pub fn run_from_cwd(phrase: &str) -> i32 {
    println!("It's okay. Cleaning up.");
    remember(phrase);
    let mut r = Report::default();
    if let Err(e) = walk(Path::new("."), &mut r) {
        eprintln!("womm: cleanup stopped early: {e}");
        return 1;
    }
    println!(
        "Removed {} kludge object(s), {} .kludged folder(s) and {} build manifest(s).",
        r.kludges, r.dirs, r.manifests
    );
    println!("Take a break. Your .womm files are still there if you ever want to come back.");
    0
}

fn walk(dir: &Path, r: &mut Report) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        // file_type() does not follow symlinks, so links are left alone.
        let ft = entry.file_type()?;

        if ft.is_dir() {
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            walk(&path, r)?;
            if name.ends_with(".kludged") && fs::read_dir(&path)?.next().is_none() {
                fs::remove_dir(&path)?;
                r.dirs += 1;
            }
        } else if ft.is_file() {
            if name.ends_with(".kludge") {
                fs::remove_file(&path)?;
                r.kludges += 1;
            } else if name.ends_with(".womm.md") && is_manifest(&path) {
                fs::remove_file(&path)?;
                r.manifests += 1;
            }
        }
    }
    Ok(())
}

fn is_manifest(path: &Path) -> bool {
    use std::io::{BufRead, BufReader};
    fs::File::open(path)
        .ok()
        .and_then(|f| BufReader::new(f).lines().next())
        .and_then(|l| l.ok())
        .is_some_and(|l| l.starts_with(MAGIC_PREFIX))
}
