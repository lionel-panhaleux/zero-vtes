//! The contract as JSON Schema: `cargo run -p zvtes-api --features schema
//! --example schema > crates/api/schema.json`; as OpenAPI with
//! `--example schema openapi > crates/api/openapi.json` (both checked by
//! `tests/schema.rs`).

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("openapi") => println!("{}", zvtes_api::openapi::json()),
        _ => println!("{}", zvtes_api::schema::json()),
    }
}
