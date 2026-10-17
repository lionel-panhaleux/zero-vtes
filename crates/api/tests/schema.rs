//! The checked-in contract files are current (`cargo test -p zvtes-api
//! --features schema`): `schema.json` and `openapi.json` what the types
//! generate (regenerate with the `schema` example), protocol §2's client
//! the runnable script.
#![cfg(feature = "schema")]

#[test]
fn the_checked_in_schema_is_current() {
    let file = include_str!("../schema.json");
    assert!(
        file.trim_end() == zvtes_api::schema::json(),
        "crates/api/schema.json is stale: cargo run -p zvtes-api --features schema \
         --example schema > crates/api/schema.json"
    );
}

#[test]
fn the_checked_in_openapi_is_current() {
    let file = include_str!("../openapi.json");
    assert!(
        file.trim_end() == zvtes_api::openapi::json(),
        "crates/api/openapi.json is stale: cargo run -p zvtes-api --features schema \
         --example schema openapi > crates/api/openapi.json"
    );
}

/// protocol §2's minimal client is the runnable `scripts/minimal_client.py`.
#[test]
fn the_minimal_client_is_the_runnable_one() {
    let doc = include_str!("../../../docs/protocol.md");
    let at = doc.find("### A minimal client").expect("§2's client");
    let start = doc[at..].find("```python\n").expect("its code") + at + "```python\n".len();
    let end = doc[start..].find("\n```").expect("its end") + start;
    let script = include_str!("../../../scripts/minimal_client.py");
    assert!(
        doc[start..end] == *script.trim_end(),
        "docs/protocol.md §2's client differs from scripts/minimal_client.py"
    );
}
