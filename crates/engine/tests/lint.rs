//! Conventions of the engine's code, checked (docs/engine.md §3).

use std::collections::HashSet;

use zvtes::crypt::CRYPT;
use zvtes::library::LIBRARY;

/// Engine files read by the lints: the hand-written ones, without the
/// definitions (where card names belong) and the generated tables.
fn engine_sources() -> Vec<(String, String)> {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let mut out = Vec::new();
    let mut dirs = vec![std::path::PathBuf::from(root)];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .to_string();
            let skip = [
                "cards/defs.rs",
                "cards/vampires.rs",
                "crypt.rs",
                "library.rs",
                "twda.rs",
                "decks.rs",
            ];
            if rel.ends_with(".rs") && !skip.contains(&rel.as_str()) {
                out.push((rel, std::fs::read_to_string(&path).unwrap()));
            }
        }
    }
    out.sort();
    out
}

/// The string literals of a source, comments and `#[cfg(test)]` modules
/// left out.
fn literals(src: &str) -> Vec<(usize, String)> {
    let code = src.split("#[cfg(test)]").next().unwrap_or(src);
    let mut out = Vec::new();
    for (n, line) in code.lines().enumerate() {
        let line = line.split("//").next().unwrap_or("");
        let mut rest = line;
        while let Some(start) = rest.find('"') {
            let after = &rest[start + 1..];
            let Some(end) = after.find('"') else { break };
            out.push((n + 1, after[..end].to_string()));
            rest = &after[end + 1..];
        }
    }
    out
}

/// Card names in engine code, each with its reason: everything else a card
/// prints lives in its definition (CLAUDE.md, docs/engine.md §4). An entry
/// no longer found fails as stale.
const CARD_NAMES_ALLOWED: &[(&str, &str, &str)] = &[
    (
        "actions.rs",
        "Mithraeum, London, The",
        "the Mithraic Mysteries membership test",
    ),
    (
        "actions.rs",
        "Mithraic Cultist",
        "the Mithraic Mysteries membership test",
    ),
    (
        "state.rs",
        "Mithras",
        "the Mithraic Mysteries membership test",
    ),
    (
        "state.rs",
        "Regent",
        "a title's name (the Regent is unique)",
    ),
    (
        "influence.rs",
        "Anarch Convert",
        "the rulebook's Anarch Convert, a crypt rule",
    ),
    (
        "ousting.rs",
        "Life Boon",
        "the oust bookkeeping it is named in",
    ),
];

#[test]
fn no_card_name_in_engine_code() {
    let names: HashSet<&str> = LIBRARY
        .iter()
        .map(|c| c.name)
        .chain(CRYPT.iter().map(|c| c.name))
        .collect();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut found = Vec::new();
    for (file, src) in engine_sources() {
        for (line, lit) in literals(&src) {
            if !names.contains(lit.as_str()) {
                continue;
            }
            seen.insert((file.clone(), lit.clone()));
            let allowed = CARD_NAMES_ALLOWED
                .iter()
                .any(|(f, n, _)| *f == file && *n == lit);
            if !allowed {
                found.push(format!("{file}:{line}: \"{lit}\""));
            }
        }
    }
    let stale: Vec<_> = CARD_NAMES_ALLOWED
        .iter()
        .filter(|(f, n, _)| !seen.contains(&(f.to_string(), n.to_string())))
        .map(|(f, n, _)| format!("{f}: \"{n}\""))
        .collect();
    assert!(
        found.is_empty(),
        "card names in engine code (move them to the card's definition, or \
         allow one with its reason in CARD_NAMES_ALLOWED):\n{}",
        found.join("\n")
    );
    assert!(stale.is_empty(), "stale entries:\n{}", stale.join("\n"));
}

/// No to-do markers in the repository's code: work found on the way is
/// done now or goes on a board through the `intake` skill (CLAUDE.md
/// "Harness"). The markers are spelled in pieces so this file passes.
#[test]
fn no_todo_in_code() {
    let repo = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let mut dirs: Vec<std::path::PathBuf> = ["crates", "scripts", "train", "deploy"]
        .iter()
        .map(|d| std::path::Path::new(repo).join(d))
        .collect();
    let markers = [
        concat!("TO", "DO"),
        concat!("FIX", "ME"),
        concat!("X", "XX"),
    ];
    let mut found = Vec::new();
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if path.is_dir() {
                if !["target", "node_modules", "build", "__pycache__"].contains(&name.as_str())
                    && !name.starts_with('.')
                {
                    dirs.push(path);
                }
                continue;
            }
            let code = [".rs", ".py", ".ts", ".mjs", ".sh"];
            if !code.iter().any(|ext| name.ends_with(ext)) {
                continue;
            }
            let Ok(src) = std::fs::read_to_string(&path) else {
                continue;
            };
            for (n, line) in src.lines().enumerate() {
                let words = line.split(|c: char| !c.is_ascii_alphanumeric());
                if words.into_iter().any(|w| markers.contains(&w)) {
                    found.push(format!("{}:{}", path.display(), n + 1));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "to-do markers in code:\n{}",
        found.join("\n")
    );
}
