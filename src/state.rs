//! One TOML state file for everything womm remembers about you.
//! Lives in `<cache>/womm/state.toml`, or `$WOMM_HOME/state.toml`.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct State {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub help_stage: u8,
    #[serde(default)]
    pub o3_streak: u32,
    #[serde(default)]
    pub cleanup: Cleanup,
    #[serde(default)]
    pub nonet: NonetState,
    #[serde(default)]
    pub bar: Bar,
    /// Day number (days since 1970) -> what happened that day. Last 7 days only.
    #[serde(default)]
    pub activity: BTreeMap<String, Day>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Cleanup {
    pub total: u32,
    pub last: u64,
    pub by_phrase: BTreeMap<String, u32>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct NonetState {
    pub epochs: u32,
    pub asks: u32,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Bar {
    pub day: u64,
    pub runs: u32,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Day {
    #[serde(default)]
    pub breakdowns: u32,
    #[serde(default)]
    pub builds: u32,
    #[serde(default)]
    pub moods: Vec<String>,
    #[serde(default)]
    pub asks: u32,
    #[serde(default)]
    pub epochs: u32,
}

/// Where womm keeps its things. `$WOMM_HOME` overrides everything (CI, tests).
pub fn home() -> Option<PathBuf> {
    if let Some(h) = std::env::var_os("WOMM_HOME") {
        return Some(PathBuf::from(h));
    }
    dirs::cache_dir().map(|d| d.join("womm"))
}

fn path() -> Option<PathBuf> {
    home().map(|h| h.join("state.toml"))
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn today() -> u64 {
    now() / 86_400
}

impl State {
    pub fn load() -> State {
        let Some(p) = path() else { return State::default() };
        match fs::read_to_string(&p) {
            Ok(text) => toml::from_str(&text).unwrap_or_else(|_| {
                eprintln!("note: i forgot everything. it's better this way.");
                State::default()
            }),
            Err(_) => {
                // Migrate v0.1's help-stage file.
                let mut s = State::default();
                if let Some(old) = home().map(|h| h.join("help-stage")) {
                    if let Ok(v) = fs::read_to_string(&old) {
                        s.help_stage = v.trim().parse().unwrap_or(0);
                        let _ = fs::remove_file(old);
                    }
                }
                s
            }
        }
    }

    /// Best effort: an unwritable cache never stops womm.
    pub fn save(&mut self) {
        self.version = 2;
        let keep = today().saturating_sub(6);
        self.activity.retain(|k, _| k.parse::<u64>().is_ok_and(|d| d >= keep));
        let Some(p) = path() else { return };
        if let Some(dir) = p.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Ok(text) = toml::to_string(self) {
            let _ = fs::write(p, text);
        }
    }

    pub fn day(&mut self, day: u64) -> &mut Day {
        self.activity.entry(day.to_string()).or_default()
    }

    pub fn day_ref(&self, day: u64) -> Day {
        self.activity.get(&day.to_string()).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_toml() {
        let mut s = State::default();
        s.cleanup.by_phrase.insert("augh".into(), 3);
        s.day(today()).moods.push("Smug".into());
        let text = toml::to_string(&s).unwrap();
        let back: State = toml::from_str(&text).unwrap();
        assert_eq!(back.cleanup.by_phrase["augh"], 3);
        assert_eq!(back.day_ref(today()).moods, vec!["Smug".to_string()]);
    }
}
