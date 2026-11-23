//! Raw-argv interception that runs before clap: the layered help and the hidden
//! cleanup commands. Nothing in this file shows up in any help output.

use std::fs;
use std::path::PathBuf;

const HELP_1: &str =
    "I mean it worked on my machine, guess it won't work on yours because my pc is arm256 haha";
const HELP_2: &str = "\"I ran just uningstool me\"";

const STOP_PHRASES: &[&str] = &[
    "i dont wanna do this anymore stop",
    "i dont want to do this anymore stop",
    "i dont wanna do this anymore",
    "i dont want to do this anymore",
    "pls stop i cant do this anymore",
    "please stop i cant do this anymore",
    "plz stop i cant do this anymore",
    "pls stop",
    "please stop",
    "make it stop",
    "i give up",
    "im pathetic",
    "i am pathetic",
    "fart",
    "stop",
];

fn normalize(args: &[String]) -> String {
    let joined = args.join(" ").to_lowercase();
    let cleaned: String = joined
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `augh`, `auuuggghhh`, `nooo`, `noooooo`, plus the fixed phrases.
pub fn is_stop(norm: &str) -> bool {
    if STOP_PHRASES.contains(&norm) {
        return true;
    }
    let w = norm.replace(' ', "");
    let runs = |s: &str, letters: &[char]| -> bool {
        let mut it = s.chars().peekable();
        for &l in letters {
            if it.peek() != Some(&l) {
                return false;
            }
            while it.peek() == Some(&l) {
                it.next();
            }
        }
        it.next().is_none()
    };
    runs(&w, &['a', 'u', 'g', 'h'])
        || (runs(&w, &['n', 'o']) && w.chars().filter(|&c| c == 'o').count() >= 2)
}

fn stage_path() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join("womm").join("help-stage"))
}

fn read_stage() -> u8 {
    stage_path()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn write_stage(stage: u8) {
    if let Some(p) = stage_path() {
        if let Some(dir) = p.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(p, stage.to_string());
    }
}

/// Returns an exit code if the args were handled here.
pub fn intercept(args: &[String]) -> Option<i32> {
    if args.is_empty() {
        println!("womm: nothing to do. It worked on my machine, though.");
        return Some(2);
    }

    let norm = normalize(args);

    if is_stop(&norm) {
        return Some(crate::cleanup::run_from_cwd());
    }

    if norm == "just uningstool me" {
        print!("{}", crate::real_help());
        write_stage(0);
        return Some(0);
    }

    let wants_help = norm == "help" || args.iter().any(|a| a == "-h" || a == "--help");
    if wants_help {
        if read_stage() == 0 {
            println!("{HELP_1}");
            write_stage(1);
        } else {
            println!("{HELP_2}");
            write_stage(2);
        }
        return Some(0);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(s: &str) -> String {
        normalize(&s.split(' ').map(String::from).collect::<Vec<_>>())
    }

    #[test]
    fn stop_variants() {
        for s in [
            "i dont wanna do this anymore stop",
            "I don't wanna do this anymore, STOP",
            "pls stop i cant do this anymore",
            "augh",
            "AUUUGGHHH",
            "nooo",
            "noooooooo",
            "fart",
            "im pathetic",
            "I'm pathetic",
        ] {
            assert!(is_stop(&n(s)), "{s} should stop");
        }
        for s in ["no", "parse", "build x", "aug", "nope"] {
            assert!(!is_stop(&n(s)), "{s} should not stop");
        }
    }
}
