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

/// Definitions whose quoted text grants a "during" ability with no ability
/// of theirs `limited` (or none: the grant a rider of a played card), each
/// with how it is once all the same. An entry no longer flagged fails as
/// stale.
const DURING_ONCE_ELSEWHERE: &[(&str, &str)] = &[
    (
        "ALAN_SOVEREIGN_ADV",
        "inert: no investment card in the pool, his base card absent",
    ),
    ("BEAST_MELD", "a prevention allowance counted down"),
    (
        "BUSINESS_PRESSURE",
        "votes bought in the card's own resolution",
    ),
    (
        "CALL_THE_WILD_HUNT",
        "the card's own damage, X set as it is played",
    ),
    ("COAGULATED_ENTITY", "a prevention allowance counted down"),
    (
        "EYES_OF_THE_WILD",
        "its burn grant removed as it is used (rules-qa 79.1)",
    ),
    (
        "FIENDISH_TONGUE",
        "a one-shot discard-phase hook, taken as used",
    ),
    ("GREENSLEEVES", "inert: Humo is not in the pool"),
    (
        "JOE_BOOT_HILL",
        "the maneuver his own action's combat grants, no ability",
    ),
    ("MONOCLE_OF_CLARITY", "no engine effect (rules-qa 68.11)"),
    (
        "MUSTAFA",
        "each older Toreador once (a forced-yet requirement)",
    ),
    (
        "NEFERTITI_ADV",
        "the [MERGED] unlock, inert without the base card",
    ),
    ("ONE_WITH_THE_LAND", "a no-strike flag set once"),
    ("ORLANDO_ORIUNDUS", "passive vote doubling, no choice"),
    ("PAULO_DE_CASTILLE", "a one-shot as he enters play"),
    (
        "PRECOGNITION",
        "a per-round prevention granted by the block",
    ),
    ("SHOW_OF_FORCE", "its strength option taken once"),
    (
        "SPECTRAL_SERVITOR",
        "one burn-unless-pool choice each unlock phase",
    ),
    ("TRAP", "automatic presses, no choice"),
    ("UNLEASH_HELLS_FURY", "a status for the action, no choice"),
];

/// Whether an ability is once in its span: a `Limit`, phase windows only
/// (once per phase by default, `Window::is_phase`), or a cost spending the
/// card or its host's unlock.
fn limited(ab: &zvtes::cards::ModeDef) -> bool {
    use zvtes::cards::Effect as E;
    ab.limit.is_some()
        || (!ab.windows.is_empty() && ab.windows.iter().all(|w| w.is_phase()))
        || ab
            .effects
            .iter()
            .any(|e| matches!(e, E::LockSelf | E::LockHost | E::BurnSelf | E::BurnSource))
}

/// "During X, … can …" is once per X (rules-digest D7, rules-qa 79.1): a
/// definition whose quoted text grants a "during" ability has an ability
/// of its own `limited`, or says how it is once in `DURING_ONCE_ELSEWHERE`.
/// It reads the `pub static` definitions (the macro families carry no
/// quoted text); it cannot tell which ability the text's sentence is, nor
/// read what an effect grants (a rider's once-ness is its exception's reason and its
/// test's, add-cards §4).
#[test]
fn during_abilities_are_once() {
    use zvtes::cards::{defs::DEFS, vampires::VAMPIRES};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/src/cards/");
    // (const, name, adv, group) of each definition granting a "during"
    // ability.
    let mut granting = Vec::new();
    for file in ["defs.rs", "vampires.rs"] {
        let src = std::fs::read_to_string(format!("{root}{file}")).unwrap();
        let lines: Vec<&str> = src.lines().collect();
        let mut doc = String::new();
        for (i, line) in lines.iter().enumerate() {
            if let Some(d) = line.strip_prefix("///") {
                doc.push(' ');
                doc.push_str(d.trim());
                continue;
            }
            if line.starts_with("#[") {
                continue;
            }
            let def = line
                .strip_prefix("pub static ")
                .filter(|l| l.contains(": CardDef = ") || l.contains(": VampireDef = "));
            let text = std::mem::take(&mut doc);
            let Some(def) = def else { continue };
            let quoted = text.split('"').nth(1).unwrap_or("");
            let grants = quoted.split(['.', ':']).any(|s| {
                let s = s.to_lowercase();
                let words: Vec<&str> = s.split(|c: char| !c.is_alphanumeric()).collect();
                words.contains(&"during")
                    && (words.contains(&"can") || words.contains(&"may"))
                    && !s.contains("only usable during")
            });
            if !grants {
                continue;
            }
            let body: Vec<&str> = lines[i..]
                .iter()
                .take_while(|l| **l != "};")
                .copied()
                .collect();
            let field = |key: &str| {
                body.iter()
                    .find_map(|l| l.trim().strip_prefix(key))
                    .map(|v| v.trim_end_matches(',').to_string())
            };
            let name = field("name: ").map(|n| n.trim_matches('"').replace("\\\"", "\""));
            granting.push((
                def.split(':').next().unwrap().to_string(),
                name.expect("a name"),
                field("adv: ").as_deref() == Some("true"),
                field("group: Some(").map(|g| g.trim_end_matches(')').to_string()),
            ));
        }
    }
    let mut flagged = Vec::new();
    for (konst, name, adv, group) in &granting {
        let abilities = DEFS
            .iter()
            .find(|d| d.name == name)
            .map(|d| d.abilities)
            .or_else(|| {
                VAMPIRES
                    .iter()
                    .find(|v| {
                        v.name == name && v.adv == *adv && v.group.map(|g| g.to_string()) == *group
                    })
                    .map(|v| v.abilities)
            });
        let abilities =
            abilities.unwrap_or_else(|| panic!("{konst}: in neither DEFS nor VAMPIRES"));
        if !abilities.iter().any(limited) {
            flagged.push(konst.clone());
        }
    }
    let unexplained: Vec<_> = flagged
        .iter()
        .filter(|n| !DURING_ONCE_ELSEWHERE.iter().any(|(e, _)| e == n))
        .collect();
    let stale: Vec<_> = DURING_ONCE_ELSEWHERE
        .iter()
        .filter(|(e, _)| !flagged.iter().any(|n| n == e))
        .map(|(e, _)| e)
        .collect();
    assert!(
        unexplained.is_empty(),
        "\"during\" abilities with no limit (rules-digest D7: once per span; \
         or say how it is once in DURING_ONCE_ELSEWHERE): {unexplained:?}"
    );
    assert!(stale.is_empty(), "stale entries: {stale:?}");
}
