//! What a womm-built executable does when run: it executes the project plan.

use std::io::Write;
use std::thread::sleep;
use std::time::Duration;

use crate::payload::Program;

pub fn run(p: Program) -> i32 {
    if std::env::args().any(|a| a == "--womm-source") {
        print!("{}", p.plan);
        return 0;
    }

    println!("{} -- built with {} from {} kludge object(s)", p.name, p.built_with, p.kludge_count);
    println!("integrity sha256:{}\n", p.source_sha256);

    let steps: Vec<&str> = p.plan.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if steps.is_empty() {
        println!("Project plan is empty. Nothing was done, efficiently.");
        return 0;
    }

    // Delays come from the source hash, so every run of a build looks the same.
    let hash = p.source_sha256.as_bytes();
    let width = steps.len().to_string().len();
    for (i, step) in steps.iter().enumerate() {
        print!("[{:>width$}/{}] {step} ... ", i + 1, steps.len());
        let _ = std::io::stdout().flush();
        let jitter = hash.get(i % hash.len().max(1)).copied().unwrap_or(0) as u64 % 16;
        sleep(Duration::from_millis(40 + jitter * 12));
        println!("done");
    }
    println!("\nProject complete. Works on my machine.");
    0
}
