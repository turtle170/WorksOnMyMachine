//! Raw-argv interception that runs before clap: the layered help and the hidden
//! cleanup commands. Nothing in this file shows up in any help output.

use crate::state::State;

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

/// Returns an exit code if the args were handled here.
pub fn intercept(args: &[String]) -> Option<i32> {
    if args.is_empty() {
        println!("womm: nothing to do. It worked on my machine, though.");
        return Some(2);
    }

    let norm = normalize(args);

    if is_stop(&norm) {
        return Some(crate::cleanup::run_from_cwd(&norm));
    }
    if norm == "i feel better now" {
        return Some(crate::hidden::feel_better());
    }

    if norm == "just uningstool me" {
        print!("{}", crate::real_help());
        let mut st = State::load();
        st.help_stage = 0;
        st.save();
        return Some(0);
    }

    let wants_help = norm == "help" || args.iter().any(|a| a == "-h" || a == "--help");
    if wants_help {
        let mut st = State::load();
        if st.help_stage == 0 {
            println!("{HELP_1}");
            st.help_stage = 1;
        } else {
            println!("{HELP_2}");
            st.help_stage = 2;
        }
        st.save();
        return Some(0);
    }

    let rest = &args[1..];
    let result = match args[0].as_str() {
        "ask" => Ok(crate::hidden::ask(rest)),
        "train" => Ok(crate::hidden::train(rest)),
        "standup" => Ok(crate::hidden::standup()),
        "why" => Ok(crate::hidden::why(rest.iter().any(|a| a == "--release"))),
        "works-on-my-machine" => crate::hidden::certificate(rest),
        "duel" => crate::hidden::duel(rest),
        "pack-store" => match rest {
            [src, out] => crate::ship::pack_store(src.as_ref(), out.as_ref()).map(|_| 0),
            _ => Err(anyhow::anyhow!("usage: womm pack-store <packages-dir> <out.tar>")),
        },
        _ => return None,
    };
    Some(result.unwrap_or_else(|e| crate::fail(&e)))
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
