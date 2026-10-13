//! The engine's version: the commit built (`ZVTES_ENGINE`), "+dirty" when
//! the tree has changes; "unknown" outside a git checkout. A build where
//! git cannot see the checkout (a container) passes it in `ZVTES_ENGINE`.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=ZVTES_ENGINE");
    if let Ok(v) = std::env::var("ZVTES_ENGINE") {
        println!("cargo:rustc-env=ZVTES_ENGINE={v}");
        return;
    }
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let commit = git(&["rev-parse", "--short=12", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let dirty =
        git(&["status", "--porcelain", "--untracked-files=no"]).is_some_and(|s| !s.is_empty());
    let version = if dirty {
        format!("{commit}+dirty")
    } else {
        commit
    };
    println!("cargo:rustc-env=ZVTES_ENGINE={version}");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/index");
}
