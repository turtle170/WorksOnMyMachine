//! Stage 2 (`build`): kludge objects -> SHA-512 -> RSA-2048 signature -> the
//! hash chain (4 more kludges, 4 more signatures, one big hash) -> NoNet's
//! extra hash -> N more kludges -> a StableHLO compute graph -> `.womm.md`,
//! followed by the garbage collector.
//!
//! Stage 3 (`parsy-builder`): verifies all five signatures, then parses the
//! final hash, very meticulously, into a Kludge VM program and links it.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha512};

use crate::gags;
use crate::grammar::Ctx;
use crate::hlo;
use crate::keys;
use crate::kludge;
use crate::nonet::{self, Voice};
use crate::payload::{self, Program};
use crate::runtime;
use crate::state::{self, State};
use crate::theatre;
use crate::vm;

pub const MAGIC_PREFIX: &str = "<!-- womm-manifest";
pub const MAGIC_LINE: &str = "<!-- womm-manifest v3 -->";
const CHAIN_LINKS: usize = 4;

fn markers(name: &str) -> (String, String) {
    (format!("<!-- womm:{name}:begin -->"), format!("<!-- womm:{name}:end -->"))
}

fn sha512_all(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut h = Sha512::new();
    for o in objects {
        h.update(o);
    }
    h.finalize().to_vec()
}

fn hex_lines(bytes: &[u8]) -> String {
    hex::encode(bytes)
        .as_bytes()
        .chunks(64)
        .map(|l| std::str::from_utf8(l).unwrap().to_string() + "\n")
        .collect()
}

fn section(md: &mut String, title: &str, marker: &str, lang: &str, body: &str) {
    let (begin, end) = markers(marker);
    md.push_str(&format!("\n## {title}\n\n{begin}\n```{lang}\n{}\n```\n{end}\n", body.trim_end()));
}

pub fn build(dir: &Path, quiet_warnings: bool) -> Result<()> {
    let (hdr, source, raw) = kludge::load(dir)?;
    let stem = hdr
        .source_name
        .strip_suffix(".womm")
        .unwrap_or(&hdr.source_name)
        .to_string();
    let text = String::from_utf8_lossy(&source).into_owned();
    let mut ctx = Ctx::from_text(&text);
    ctx.kludges = hdr.total as usize;

    println!("   Compiling {} kludge object(s) from {}", hdr.total, dir.display());
    let key = keys::machine_key()?;
    let der = keys::public_der(&key);

    // The compiler quirk: NoNet is constructed first, and trained on womm itself.
    println!("   Constructing NoNet ({} parameters, 3.33 KB)", nonet::PARAMS);
    let compiler_seed = [b"compiler:".as_slice(), &der].concat();
    let net = nonet::compiler_nonet(&compiler_seed);
    println!("   Tuned NoNet with AdamW on the compiler binary (it does something now)");

    // Link 0: the classic signature.
    let digest = sha512_all(&raw);
    let sig0 = keys::sign(&key, &digest)?;
    let mut voice = Voice::with_net(net.clone(), &sig0);
    theatre::lying_bar("Signing kludge", &mut voice);
    println!("   Digest SHA-512 {}", hex::encode(&digest[..16]));

    gags::kludge_rot(dir, &mut voice, &ctx);
    let earlier = gags::warnings(
        &mut Voice::with_net(net.clone(), &gags::parse_seed(&der, &hdr.source_sha256)),
        &text,
        hdr.opt_level,
    );
    if !quiet_warnings && !earlier.is_empty() {
        println!("   re-analysing with signature... previous warnings were preliminary.");
    }
    let ws = gags::warnings(&mut voice, &text, hdr.opt_level);
    gags::print_warnings(&ws, Some(&earlier), quiet_warnings);

    // The hash chain: the signature becomes 4 kludge objects, each signed again.
    let chain_dir = dir.join("chain.kludged");
    kludge::prepare_dir(&chain_dir)?;
    kludge::emit(&chain_dir, "signature.sig", &sig0, CHAIN_LINKS as u32, hdr.opt_level)?;
    let (_, _, chain_raw) = kludge::load(&chain_dir)?;
    let mut chain_digests = Vec::new();
    let mut big = Vec::new();
    for object in &chain_raw {
        let d = Sha512::digest(object).to_vec();
        big.extend(keys::sign(&key, &d)?);
        chain_digests.push(d);
    }
    theatre::lying_bar("Signing chain", &mut voice);
    println!("   Hash chain: 4 kludge objects -> 4 signatures -> {}-byte big hash", big.len());

    // NoNet reads the big hash and adds its own.
    let extra = net.extra_hash(&big);
    let final_hash = [big.as_slice(), &extra].concat();
    println!("   NoNet appended a {}-byte extra hash ({} bytes total)", extra.len(), final_hash.len());

    // ...which becomes more kludge, which is parsed.
    let nonet_dir = dir.join("nonet.kludged");
    kludge::prepare_dir(&nonet_dir)?;
    let n = final_hash.len().div_ceil(64) as u32;
    kludge::emit(&nonet_dir, "final.hash", &final_hash, n, hdr.opt_level)?;
    let (_, reparsed, _) = kludge::load(&nonet_dir)?;
    if reparsed != final_hash {
        bail!("final hash did not survive being kludged");
    }
    println!("   Re-kludged into {n} object(s) and parsed them back");

    // ...which is lowered to an XLA compute graph.
    let module = hlo::emit(&net, &big);
    println!("   Lowered to StableHLO ({} lines)", module.lines().count());

    let fingerprint = keys::fingerprint(&key);
    let mut md = String::new();
    md.push_str(MAGIC_LINE);
    md.push_str(&format!("\n# Build Manifest: {stem}\n\n"));
    md.push_str("> Generated by womm. This document is the build output. Do not edit.\n\n");
    md.push_str("## Parameters\n\n| Key | Value |\n|---|---|\n");
    let mut params: Vec<(String, String)> = vec![
        ("name".into(), stem.clone()),
        ("source".into(), hdr.source_name.clone()),
        ("source_sha256".into(), hdr.source_sha256.clone()),
        ("parser_opt_level".into(), hdr.opt_level.to_string()),
        ("kludge_count".into(), hdr.total.to_string()),
        ("kludge_sha512".into(), hex::encode(&digest)),
    ];
    for (i, d) in chain_digests.iter().enumerate() {
        params.push((format!("chain_sha512_{i}"), hex::encode(d)));
    }
    params.extend([
        ("signature_scheme".into(), keys::SCHEME.to_string()),
        ("key_fingerprint".into(), fingerprint),
        ("nonet_parameters".into(), nonet::PARAMS.to_string()),
        ("extra_hash_len".into(), extra.len().to_string()),
        ("final_hash_len".into(), final_hash.len().to_string()),
        ("final_kludge_count".into(), n.to_string()),
        ("compiler".into(), format!("womm {}", env!("CARGO_PKG_VERSION"))),
    ]);
    for (k, v) in params {
        md.push_str(&format!("| {k} | `{v}` |\n"));
    }
    section(&mut md, "Signature", "signature", "hex", &hex_lines(&sig0));
    section(&mut md, "Hash chain", "chain", "hex", &hex_lines(&big));
    section(&mut md, "Extra hash (NoNet)", "extra", "hex", &hex_lines(&extra));
    section(&mut md, "Compute graph (StableHLO)", "stablehlo", "mlir", &module);
    section(&mut md, "Public key", "pubkey", "pem", &keys::public_pem(&key)?);

    let out = dir
        .parent()
        .unwrap_or(Path::new("."))
        .join(format!("{stem}.womm.md"));
    fs::write(&out, md).with_context(|| format!("could not write {}", out.display()))?;

    theatre::garbage_collector(&mut voice, &ctx, &[chain_dir, nonet_dir]);
    println!("   Wrote build manifest {}", out.display());
    println!("    Finished. Next: womm parsy-builder {}", out.display());
    Ok(())
}

pub fn param(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let cells: Vec<&str> = l.split('|').map(str::trim).collect();
        (cells.len() >= 3 && cells[1] == key).then(|| cells[2].trim_matches('`').to_string())
    })
}

/// The lines between a section's markers, without the code fences.
pub fn block(text: &str, name: &str) -> Result<Vec<String>> {
    let (begin, end) = markers(name);
    let start = text.find(&begin).with_context(|| format!("manifest has no {name} section"))? + begin.len();
    let stop = text[start..]
        .find(&end)
        .with_context(|| format!("manifest {name} section is unterminated"))?
        + start;
    Ok(text[start..stop]
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with("```"))
        .map(String::from)
        .collect())
}

/// Lex a hex section very meticulously: every character is checked.
fn lex_hex(text: &str, name: &str) -> Result<Vec<u8>> {
    let lines = block(text, name)?;
    for (n, l) in lines.iter().enumerate() {
        if let Some((col, c)) = l.char_indices().find(|(_, c)| !c.is_ascii_hexdigit()) {
            bail!("{name} line {}, column {}: illegal character {c:?}", n + 1, col + 1);
        }
    }
    hex::decode(lines.concat()).with_context(|| format!("{name} has an odd number of digits"))
}

pub fn read_manifest(manifest: &Path) -> Result<String> {
    let text = fs::read_to_string(manifest)
        .with_context(|| format!("could not read {}", manifest.display()))?;
    let first = text.lines().next().unwrap_or("");
    if !first.starts_with(MAGIC_PREFIX) {
        bail!("{} is not a womm build manifest", manifest.display());
    }
    if first != MAGIC_LINE {
        bail!("{} was built by an older womm; run `womm build` again", manifest.display());
    }
    Ok(text)
}

fn digest_param(text: &str, key: &str) -> Result<Vec<u8>> {
    let d = hex::decode(param(text, key).with_context(|| format!("manifest has no {key}"))?)
        .with_context(|| format!("{key} is not hexadecimal"))?;
    if d.len() != 64 {
        bail!("{key} must be 64 bytes, found {}", d.len());
    }
    Ok(d)
}

pub fn link(manifest: &Path, output: Option<PathBuf>) -> Result<()> {
    let text = read_manifest(manifest)?;
    println!("   Reading manifest {}", manifest.display());

    let name = param(&text, "name").context("manifest has no name")?;
    let opt_level: u8 = param(&text, "parser_opt_level").and_then(|v| v.parse().ok()).unwrap_or(0);
    let kludge_count: u32 = param(&text, "kludge_count").and_then(|v| v.parse().ok()).unwrap_or(0);
    let public_pem = block(&text, "pubkey")?.join("\n") + "\n";
    let fingerprint = keys::fingerprint_of_pem(&public_pem)?;

    // Stage 1: lexing.
    let sig0 = lex_hex(&text, "signature")?;
    let big = lex_hex(&text, "chain")?;
    let extra = lex_hex(&text, "extra")?;
    println!(
        "   Lexed {} hexadecimal digits across 3 sections",
        2 * (sig0.len() + big.len() + extra.len())
    );

    // Stage 2: authentication, five times.
    keys::verify(&public_pem, &digest_param(&text, "kludge_sha512")?, &sig0)?;
    if big.len() != CHAIN_LINKS * 256 {
        bail!("hash chain must be {} bytes, found {}", CHAIN_LINKS * 256, big.len());
    }
    for (i, sig) in big.chunks(256).enumerate() {
        keys::verify(&public_pem, &digest_param(&text, &format!("chain_sha512_{i}"))?, sig)
            .with_context(|| format!("chain link {i}"))?;
    }
    println!("   Verified 5 RSA-2048 signatures (key {fingerprint})");

    // Stage 3: decoding.
    let final_hash = [big.as_slice(), &extra].concat();
    if param(&text, "final_hash_len").and_then(|v| v.parse::<usize>().ok()) != Some(final_hash.len()) {
        bail!("final hash length does not match the manifest");
    }
    let signature = hex::encode(&final_hash);
    let code = vm::decode_program(&signature)?;
    println!("   Paired {} nibbles into {} instructions", signature.len(), code.len());

    // Stage 4: analysis.
    let a = vm::analyze(&code);
    let mut hist: Vec<String> = vm::MNEMONICS
        .iter()
        .zip(a.histogram)
        .filter(|(_, n)| *n > 0)
        .map(|(m, n)| format!("{m}x{n}"))
        .collect();
    hist.sort();
    println!("   Instruction mix: {}", hist.join(" "));
    println!(
        "   Control flow: loop depth {}, {} unterminated loop(s), {} stray END(s), {}",
        a.max_depth,
        a.unterminated_loops,
        a.stray_ends,
        match a.halt_at {
            Some(pc) => format!("HALT at {pc:#06x}"),
            None => "no HALT (runs to completion)".to_string(),
        }
    );
    let mood = runtime::Mood::of(&code);
    println!("   Personality: {} (this was not a choice)", mood.name());

    // Stage 5: linking.
    let program = Program {
        name: name.clone(),
        signature,
        key_fingerprint: fingerprint,
        opt_level,
        kludge_count,
        built_with: format!("womm {}", env!("CARGO_PKG_VERSION")),
        host_hash: keys::host_hash(),
        arch: std::env::consts::ARCH.to_string(),
    };
    let exe = payload::assemble(&payload::base_stub()?, &program)?;

    let out = output.unwrap_or_else(|| {
        let file = format!("{name}{}", std::env::consts::EXE_SUFFIX);
        manifest.parent().unwrap_or(Path::new(".")).join(file)
    });
    fs::write(&out, &exe).with_context(|| format!("could not write {}", out.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&out, fs::Permissions::from_mode(0o755))?;
    }

    let mut st = State::load();
    let day = st.day(state::today());
    day.builds += 1;
    day.moods.push(mood.name().to_string());
    st.save();

    println!("   Linked {} ({} bytes)", out.display(), exe.len());
    println!("    Finished. Your project is now executable.");
    Ok(())
}
