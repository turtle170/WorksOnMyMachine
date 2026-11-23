//! The one useful feature: remove every .kludge object, empty .kludged
//! directory and intermediate .womm.md manifest under the current directory.
//! Source .womm files and built executables are never touched.

use std::fs;
use std::path::Path;

use crate::manifest::MAGIC_LINE;

#[derive(Default)]
struct Report {
    kludges: usize,
    dirs: usize,
    manifests: usize,
}

pub fn run_from_cwd() -> i32 {
    println!("It's okay. Cleaning up.");
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
        .is_some_and(|l| l == MAGIC_LINE)
}
