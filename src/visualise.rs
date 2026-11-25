//! `womm visualise`: prints an eye. Also, for once, does something real: it
//! translates the manifest's StableHLO graph into readable Python.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::hlo;
use crate::manifest;
use crate::ship;

fn bundled_python() -> Vec<PathBuf> {
    let store = ship::store_dir();
    vec![
        store.join("python").join("bin").join("python3"),
        store.join("python").join("python.exe"),
        PathBuf::from("python3"),
        PathBuf::from("python"),
    ]
}

pub fn visualise(path: &Path, run: bool) -> Result<i32> {
    println!("\u{1F441}");
    let text = manifest::read_manifest(path)?;
    let module = manifest::block(&text, "stablehlo")?.join("\n");
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("manifest");
    let py = hlo::python(&module, name)?;
    let stem = name.strip_suffix(".womm.md").unwrap_or(name);
    let out = path.parent().unwrap_or(Path::new(".")).join(format!("{stem}.nonet.py"));
    fs::write(&out, py).with_context(|| format!("could not write {}", out.display()))?;
    println!("   (also wrote {}: NoNet, in Python, for humans)", out.display());

    if !run {
        return Ok(0);
    }
    if !ship::installed("nonet") {
        println!("No.");
        return Ok(0);
    }
    let expected = manifest::block(&text, "extra")?.concat();
    for py in bundled_python() {
        let Ok(o) = Command::new(&py).arg(&out).output() else { continue };
        if !o.status.success() {
            continue;
        }
        let got = String::from_utf8_lossy(&o.stdout).trim().to_string();
        println!("{got}");
        if got == expected {
            println!("   matches the manifest. NoNet is real.");
            return Ok(0);
        }
        println!("   does not match the manifest. it worked on my machine.");
        return Ok(1);
    }
    anyhow::bail!("no Python found (the Windows installer bundles one; elsewhere, put python3 on your PATH)")
}
