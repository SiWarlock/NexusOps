//! Emits the versioned JSON-Schema artifact from the Rust contract authority
//! (ARCHITECTURE §5.0). Run `cargo run --bin emit_schema` after any contract
//! change, then commit the regenerated file (test 9 fails on drift).

fn main() {
    // Build-time tool: expect() here is correct fail-closed behavior — a broken
    // build must abort, and this never runs on a daemon runtime/request path.
    let json = nexusops_shared::schema::emit_schema_json();
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/contracts/schema");
    std::fs::create_dir_all(dir).expect("create contracts/schema dir");
    let path = format!("{dir}/nexusops-contract.schema.json");
    std::fs::write(&path, json).expect("write schema artifact");
    println!("wrote {path}");
}
