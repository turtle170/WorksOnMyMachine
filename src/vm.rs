//! The Kludge Virtual Machine.
//!
//! A program is the final hash: one instruction per byte, `oooo aaaa`, where the
//! high nibble picks the opcode and the low nibble is its argument. Programs
//! vary in length (NoNet decides). Every program is valid and every program
//! terminates. Longer programs are allowed to say more.

use std::io::Write;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{bail, Result};

/// Output and step budgets are granted per 256 instructions.
pub const BUDGET_UNIT: usize = 256;
pub const MAX_PROGRAM_LEN: usize = 4096;
const MAX_STEPS: usize = 20_000;
const MAX_LINES: usize = 120;
const WRAP: usize = 64;
const MAX_LOOP_DEPTH: usize = 4;

pub const MNEMONICS: [&str; 16] = [
    "GLYPH", "REPEAT", "COLOR", "NEWLINE", "GLYPHSET", "LOOP", "END", "WORD", "SLEEP", "ADD",
    "GLYPHR", "SPACE", "STRIPE", "EXCUSE", "SKIP", "MIRROR",
];

/// Each set has exactly 16 single-width glyphs.
pub const GLYPH_SETS: [&str; 6] = [
    "░▒▓█▀▄▌▐■□▪▫▲▼◆◇",
    "─│┌┐└┘├┤┬┴┼═║╔╗╝",
    "·•∘○◎●◌◍◐◑◒◓◔◕◖◗",
    "#*+=-~^%&@$/\\|<>",
    "⠁⠂⠄⠈⠐⠠⡀⢀⠃⠅⠉⠑⠡⣀⣿⠿",
    "~-_=^vVwWmMnNuUo",
];

const WORDS: [&str; 32] = [
    "synergy", "leverage", "pivot", "paradigm", "blockchain", "agile", "scalable",
    "cloud-native", "disrupt", "bandwidth", "deliverable", "stakeholder", "roadmap",
    "alignment", "velocity", "holistic", "ecosystem", "bespoke", "frictionless", "circle back",
    "deep dive", "low-hanging fruit", "move the needle", "north star", "boil the ocean", "ping",
    "sprint", "backlog", "KPI", "quick win", "10x", "AI",
];

const EXCUSES: [&str; 16] = [
    "it worked on my machine",
    "must be a caching issue",
    "did you clear your cache?",
    "that's a feature",
    "works in staging",
    "the tests passed locally",
    "cosmic ray bit flip",
    "it's a DNS thing",
    "probably your PATH",
    "Mercury is in retrograde",
    "it's Tuesday somewhere",
    "the intern did it",
    "cannot reproduce",
    "known issue, wontfix",
    "have you tried turning it off and on again?",
    "upstream bug",
];

fn glyph(set: usize, i: usize) -> char {
    GLYPH_SETS[set % GLYPH_SETS.len()].chars().nth(i % 16).unwrap()
}

/// The line budget for a program of this length.
pub fn max_lines(len: usize) -> usize {
    MAX_LINES * len.div_ceil(BUDGET_UNIT).max(1)
}

/// Strictly decode a hex program: an even number of hex digits, at least one
/// instruction, at most MAX_PROGRAM_LEN.
pub fn decode_program(hex: &str) -> Result<Vec<u8>> {
    if hex.is_empty() || !hex.len().is_multiple_of(2) || hex.len() > MAX_PROGRAM_LEN * 2 {
        bail!(
            "expected an even number of hex digits between 2 and {}, found {}",
            MAX_PROGRAM_LEN * 2,
            hex.len()
        );
    }
    if let Some((i, c)) = hex.char_indices().find(|(_, c)| !c.is_ascii_hexdigit()) {
        bail!("illegal character {c:?} at column {i}; signatures are hexadecimal");
    }
    Ok(hex::decode(hex)?)
}

pub struct Analysis {
    pub histogram: [usize; 16],
    pub max_depth: usize,
    pub unterminated_loops: usize,
    pub stray_ends: usize,
    pub halt_at: Option<usize>,
}

/// Static analysis of a program, for the compiler's benefit (and yours).
pub fn analyze(code: &[u8]) -> Analysis {
    let mut a = Analysis {
        histogram: [0; 16],
        max_depth: 0,
        unterminated_loops: 0,
        stray_ends: 0,
        halt_at: None,
    };
    let mut depth = 0usize;
    for (pc, &b) in code.iter().enumerate() {
        a.histogram[(b >> 4) as usize] += 1;
        match b >> 4 {
            0x5 => {
                depth += 1;
                a.max_depth = a.max_depth.max(depth.min(MAX_LOOP_DEPTH));
            }
            0x6 if depth > 0 => depth -= 1,
            0x6 => a.stray_ends += 1,
            _ if b == 0xFF && a.halt_at.is_none() => a.halt_at = Some(pc),
            _ => {}
        }
    }
    a.unterminated_loops = depth;
    a
}

pub fn disasm(code: &[u8]) -> String {
    let mut s = String::new();
    for (pc, &b) in code.iter().enumerate() {
        let op = MNEMONICS[(b >> 4) as usize];
        let op = if b == 0xFF { "HALT" } else { op };
        s.push_str(&format!("{pc:04x}  {b:02x}  {op:<8} {}\n", b & 0xF));
    }
    s
}

struct Machine<'a> {
    out: &'a mut dyn Write,
    max_lines: usize,
    color: bool,
    col: usize,
    lines: usize,
    set: usize,
    glyph: char,
    reg: usize,
}

impl Machine<'_> {
    fn full(&self) -> bool {
        self.lines >= self.max_lines
    }
    fn emit(&mut self, s: &str) {
        if self.full() {
            return;
        }
        let _ = self.out.write_all(s.as_bytes());
        self.col += s.chars().count();
        if self.col >= WRAP {
            self.newline();
        }
    }
    fn emit_glyph(&mut self, c: char, n: usize) {
        let mut buf = [0u8; 4];
        for _ in 0..n {
            self.emit(c.encode_utf8(&mut buf));
        }
    }
    fn newline(&mut self) {
        if self.full() {
            return;
        }
        let _ = self.out.write_all(b"\n");
        let _ = self.out.flush();
        self.col = 0;
        self.lines += 1;
    }
}

pub fn run(code: &[u8], out: &mut dyn Write, animate: bool, color: bool) {
    let max_lines = max_lines(code.len());
    let max_steps = MAX_STEPS * code.len().div_ceil(BUDGET_UNIT).max(1);
    let mut m = Machine { out, max_lines, color, col: 0, lines: 0, set: 0, glyph: glyph(0, 0), reg: 0 };
    let mut loops: Vec<(usize, usize)> = Vec::new(); // (body start, iterations left)
    let (mut pc, mut steps) = (0usize, 0usize);

    while pc < code.len() && steps < max_steps && !m.full() {
        steps += 1;
        let (op, a) = (code[pc] >> 4, (code[pc] & 0xF) as usize);
        let mut next = pc + 1;
        match op {
            0x0 => {
                m.glyph = glyph(m.set, a);
                m.emit_glyph(m.glyph, 1);
            }
            0x1 => m.emit_glyph(m.glyph, a % 8 + 1),
            0x2 => {
                if m.color {
                    let sgr = if a >= 8 { 90 + a % 8 } else { 30 + a };
                    let _ = write!(m.out, "\x1b[{sgr}m");
                }
            }
            0x3 => {
                m.newline();
                if a >= 8 && m.color {
                    let _ = m.out.write_all(b"\x1b[0m");
                }
            }
            0x4 => m.set = a % GLYPH_SETS.len(),
            0x5 => {
                if loops.len() < MAX_LOOP_DEPTH {
                    loops.push((next, a % 3 + 1));
                }
            }
            0x6 => {
                if let Some(top) = loops.last_mut() {
                    if top.1 > 0 {
                        top.1 -= 1;
                        next = top.0;
                    } else {
                        loops.pop();
                    }
                }
            }
            0x7 => {
                let w = WORDS[(a + m.reg) % WORDS.len()];
                m.emit(&format!(" {w} "));
            }
            0x8 => {
                if animate {
                    let _ = m.out.flush();
                    sleep(Duration::from_millis(a as u64 * 20));
                }
            }
            0x9 => m.reg = m.reg.wrapping_add(a + 1),
            0xA => {
                m.glyph = glyph(m.set, m.reg.wrapping_add(a));
                m.emit_glyph(m.glyph, 1);
            }
            0xB => m.emit(&" ".repeat(a % 6 + 1)),
            0xC => {
                let g = glyph(m.set, a);
                m.emit_glyph(g, 8);
            }
            0xD => {
                if m.col > 0 {
                    m.newline();
                }
                let e = EXCUSES[(a + m.reg) % EXCUSES.len()];
                m.emit(&format!("  // {e}"));
                m.newline();
            }
            0xE => next = pc + 1 + a % 3,
            _ => {
                if a == 0xF {
                    break;
                }
                m.glyph = glyph(m.set, 15 - a);
                m.emit_glyph(m.glyph, 1);
            }
        }
        pc = next;
    }

    if m.color {
        let _ = m.out.write_all(b"\x1b[0m");
    }
    if m.col > 0 {
        m.newline();
    }
    let _ = m.out.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_sets_have_sixteen_glyphs() {
        for s in GLYPH_SETS {
            assert_eq!(s.chars().count(), 16, "{s}");
        }
    }

    #[test]
    fn decode_is_strict() {
        assert!(decode_program(&"ab".repeat(256)).is_ok());
        assert!(decode_program(&"ab".repeat(1100)).is_ok());
        assert!(decode_program("abc").is_err());
        assert!(decode_program("").is_err());
        assert!(decode_program(&format!("zz{}", "ab".repeat(255))).is_err());
    }

    #[test]
    fn every_program_terminates_deterministically() {
        // A cheap LCG gives a spread of programs, including loop-heavy ones.
        let mut x: u32 = 0x5eed;
        for n in 0..200 {
            let len = [256, 1040, 1152, 3][n % 4];
            let code: Vec<u8> = (0..len)
                .map(|_| {
                    x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                    (x >> 16) as u8
                })
                .collect();
            let (mut a, mut b) = (Vec::new(), Vec::new());
            run(&code, &mut a, false, true);
            run(&code, &mut b, false, true);
            assert_eq!(a, b);
            assert!(a.iter().filter(|&&b| b == b'\n').count() <= max_lines(len));
            assert!(String::from_utf8(a).is_ok());
        }
        let all_loops = vec![0x52u8; 1152];
        run(&all_loops, &mut Vec::new(), false, false);
    }
}
