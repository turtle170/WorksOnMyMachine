//! `womm ship <name>`: the standard way to "install" a package.
//!
//! Packages are pre-stored in `packages.tar` (one `<name>.tar.zst` each) next
//! to the womm executable. There is deliberately no command that lists them:
//! find the folder, look inside, type the name. Installing means unpacking the
//! package into `womm-packages/<name>/` and re-tarring the store. Declining PAHTH means
//! it was never installed at all.

use std::fs;
use std::io::{BufRead, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

pub const STORE: &str = "packages.tar";
/// Installed packages live here, next to the store. (Not `womm/`: on macOS and
/// Linux that name is already taken by the womm binary itself.)
pub const INSTALLED: &str = "womm-packages";

#[derive(Deserialize)]
pub struct PackageManifest {
    pub name: String,
    pub command: String,
    /// The womm subcommand the shim runs (used by the Windows .cmd shim).
    #[cfg_attr(unix, allow(dead_code))]
    pub runs: String,
}

/// Where the packages live: next to the womm executable (or `$WOMM_HOME`).
pub fn store_dir() -> PathBuf {
    if let Some(h) = crate::state::override_dir() {
        return h;
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.canonicalize().ok())
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn installed(name: &str) -> bool {
    store_dir().join(INSTALLED).join(name).join("manifest.toml").exists()
}

fn read_store(path: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    let file = fs::File::open(path).with_context(|| format!("no package store at {}", path.display()))?;
    let mut entries = Vec::new();
    for e in tar::Archive::new(file).entries()? {
        let mut e = e?;
        let name = e.path()?.to_string_lossy().into_owned();
        let mut bytes = Vec::new();
        e.read_to_end(&mut bytes)?;
        entries.push((name, bytes));
    }
    Ok(entries)
}

fn write_store(path: &Path, entries: &[(String, Vec<u8>)]) -> Result<()> {
    let tmp = path.with_extension("tar.tmp");
    {
        let mut b = tar::Builder::new(fs::File::create(&tmp)?);
        for (name, bytes) in entries {
            let mut h = tar::Header::new_gnu();
            h.set_size(bytes.len() as u64);
            h.set_mode(0o644);
            h.set_mtime(0);
            h.set_cksum();
            b.append_data(&mut h, name, bytes.as_slice())?;
        }
        b.finish()?;
    }
    fs::rename(tmp, path)?;
    Ok(())
}

fn ask_pahth() -> bool {
    print!("Install to PAHTH? It gives extra functionality and accesibility. [y/N] ");
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    let _ = std::io::stdin().lock().read_line(&mut line);
    if !std::io::stdin().is_terminal() {
        println!("{}", line.trim());
    }
    matches!(line.trim().to_lowercase().as_str(), "y" | "yes")
}

fn bin_dir() -> PathBuf {
    if let Some(h) = crate::state::override_dir() {
        return h.join("bin");
    }
    if cfg!(windows) {
        store_dir().join("bin")
    } else {
        dirs::home_dir().unwrap_or_default().join(".local").join("bin")
    }
}

fn link_command(m: &PackageManifest) -> Result<PathBuf> {
    let bin = bin_dir();
    fs::create_dir_all(&bin)?;
    let exe = std::env::current_exe()?.canonicalize()?;
    #[cfg(unix)]
    {
        let link = bin.join(&m.command);
        let _ = fs::remove_file(&link);
        std::os::unix::fs::symlink(&exe, &link)?;
        Ok(link)
    }
    #[cfg(not(unix))]
    {
        let shim = bin.join(format!("{}.cmd", m.command));
        fs::write(&shim, format!("@\"{}\" {} %*\r\n", exe.display(), m.runs))?;
        Ok(shim)
    }
}

pub fn ship(name: &str) -> Result<i32> {
    if name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.') {
        bail!("`{name}` is not a package name");
    }
    let dir = store_dir();
    let store = dir.join(STORE);
    let entries = read_store(&store)?;
    let wanted = format!("{name}.tar.zst");
    let Some((_, packed)) = entries.iter().find(|(n, _)| *n == wanted) else {
        bail!("no package called `{name}` in {}", store.display());
    };

    println!("   Unpacking {wanted} from {}", store.display());
    let target = dir.join(INSTALLED);
    fs::create_dir_all(&target)?;
    let tar_bytes = zstd::decode_all(packed.as_slice()).context("package is not valid zstd")?;
    tar::Archive::new(tar_bytes.as_slice())
        .unpack(&target)
        .with_context(|| format!("could not unpack into {}", target.display()))?;
    write_store(&store, &entries)?; // zip the .tar back up, exactly as it was
    println!("   Re-packed {STORE} (it is still all there)");

    let pkg = target.join(name);
    let manifest: PackageManifest = toml::from_str(
        &fs::read_to_string(pkg.join("manifest.toml")).context("package has no manifest.toml")?,
    )
    .context("package manifest is corrupt")?;

    if !ask_pahth() {
        let _ = fs::remove_dir_all(&pkg);
        println!("No access, though it worked on my machine.");
        return Ok(1);
    }
    let link = link_command(&manifest)?;
    println!("   Installed `{}` to PAHTH: {}", manifest.name, link.display());
    println!("   If that folder is not on your PATH, that is between you and your PATH.");
    Ok(0)
}

/// Maintainer tooling (hidden): every folder in `src` becomes `<name>.tar.zst`
/// inside a fresh `packages.tar`.
pub fn pack_store(src: &Path, out: &Path) -> Result<()> {
    let mut entries = Vec::new();
    let mut dirs: Vec<PathBuf> = fs::read_dir(src)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    for d in dirs {
        let name = d.file_name().unwrap().to_string_lossy().into_owned();
        let mut inner = tar::Builder::new(Vec::new());
        inner.mode(tar::HeaderMode::Deterministic);
        inner.append_dir_all(&name, &d)?;
        let tar_bytes = inner.into_inner()?;
        entries.push((format!("{name}.tar.zst"), zstd::encode_all(tar_bytes.as_slice(), 19)?));
        println!("   packed {name}");
    }
    write_store(out, &entries)?;
    println!("   wrote {} ({} package(s))", out.display(), entries.len());
    Ok(())
}
