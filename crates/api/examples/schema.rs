//! The contract as JSON Schema: `cargo run -p zvtes-api --features schema
//! --example schema > crates/api/schema.json` (checked by `tests/schema.rs`).

fn main() {
    println!("{}", zvtes_api::schema::json());
}
