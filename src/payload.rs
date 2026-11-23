//! The womm binary doubles as the runtime stub. A built program is
//! `[womm binary][TOML payload][u64 LE payload length][MAGIC]`.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const MAGIC: &[u8; 8] = b"WOMMKLDG";
const FOOTER: usize = 16;

#[derive(Serialize, Deserialize)]
pub struct Program {
    pub name: String,
    /// 512 hex digits: the RSA-2048 signature, which is also the program.
    pub signature: String,
    pub key_fingerprint: String,
    pub opt_level: u8,
    pub kludge_count: u32,
    pub built_with: String,
}

/// If the running executable carries a payload, return it.
pub fn read_self() -> Option<Program> {
    let exe = std::env::current_exe().ok()?;
    let mut f = File::open(exe).ok()?;
    let len = f.metadata().ok()?.len();
    if len < FOOTER as u64 {
        return None;
    }
    f.seek(SeekFrom::End(-(FOOTER as i64))).ok()?;
    let mut footer = [0u8; FOOTER];
    f.read_exact(&mut footer).ok()?;
    if &footer[8..] != MAGIC {
        return None;
    }
    let plen = u64::from_le_bytes(footer[..8].try_into().ok()?);
    if plen > len - FOOTER as u64 {
        return None;
    }
    f.seek(SeekFrom::End(-(FOOTER as i64) - plen as i64)).ok()?;
    let mut buf = vec![0u8; plen as usize];
    f.read_exact(&mut buf).ok()?;
    toml::from_str(std::str::from_utf8(&buf).ok()?).ok()
}

/// The current executable with any existing payload removed.
pub fn base_stub() -> Result<Vec<u8>> {
    let exe = std::env::current_exe().context("could not locate the womm executable")?;
    let bytes = std::fs::read(&exe).context("could not read the womm executable")?;
    Ok(strip(bytes))
}

fn strip(mut bytes: Vec<u8>) -> Vec<u8> {
    let n = bytes.len();
    if n >= FOOTER && &bytes[n - 8..] == MAGIC {
        let plen = u64::from_le_bytes(bytes[n - 16..n - 8].try_into().unwrap()) as usize;
        if plen <= n - FOOTER {
            bytes.truncate(n - FOOTER - plen);
        }
    }
    bytes
}

pub fn assemble(stub: &[u8], program: &Program) -> Result<Vec<u8>> {
    let payload = toml::to_string(program)?;
    let mut out = Vec::with_capacity(stub.len() + payload.len() + FOOTER);
    out.extend_from_slice(stub);
    out.extend_from_slice(payload.as_bytes());
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    out.extend_from_slice(MAGIC);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assemble_then_strip_round_trips() {
        let stub = b"pretend this is a binary".to_vec();
        let p = Program {
            name: "x".into(),
            signature: "ab".repeat(256),
            key_fingerprint: "00".into(),
            opt_level: 0,
            kludge_count: 4,
            built_with: "test".into(),
        };
        let exe = assemble(&stub, &p).unwrap();
        assert_eq!(strip(exe), stub);
    }
}
