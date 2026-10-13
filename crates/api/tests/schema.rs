//! `schema.json` is what the types generate (`cargo test -p zvtes-api
//! --features schema`; regenerate with the `schema` example).
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
