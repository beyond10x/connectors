# Legacy decoder verification harness

This temporary Rust program links the unchanged current connectors-core crate. It exercises existing decoding only, and implements no new codec. Source SHA-256: `b91f3bd7124e672eeaa5081d4dad1b130e4bc22baa41ffd8af211893c23fdcf0`.

The local scratch Cargo package has an empty `[workspace]`, dependencies `connectors-core` pointing at this checkout and `serde_json = "1"`. Its lock starts from the root Cargo.lock. Run with the checked-in byte vectors:

```sh
env TMPDIR="$PWD/.local/tmp" CARGO_TARGET_DIR="$PWD/target" cargo run --offline --quiet --manifest-path .local/wire-compatibility-20260908/probe/Cargo.toml -- docs/evidence/wire-compatibility-20260908/legacy-decoder-vectors.json
```

```rust
use connectors_core::{read_json, Descriptor, Error, Invocation, Operation, Response};
use serde_json::{json, Value};
fn main() {
    let path = std::env::args().nth(1).expect("vectors path");
    let bytes = std::fs::read(path).unwrap();
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    let mut results = Vec::new();
    for vector in document["vectors"].as_array().unwrap() {
        let bytes = vector["bytes"].as_str().unwrap().as_bytes();
        let accepted = match vector["type"].as_str().unwrap() {
            "descriptor" => read_json::<Descriptor>(bytes).is_ok(),
            "operation" => read_json::<Operation>(bytes).is_ok(),
            "invocation" => read_json::<Invocation>(bytes).is_ok(),
            "response" => read_json::<Response>(bytes).is_ok(),
            "error" => read_json::<Error>(bytes).is_ok(),
            other => panic!("unrecognized probe type {other}"),
        };
        assert_eq!(accepted, vector["expected_decode"].as_bool().unwrap(), "{}", vector["id"]);
        results.push(json!({"id": vector["id"], "accepted": accepted, "passed": true}));
    }
    println!("{}", serde_json::to_string_pretty(&json!({"passed": results.len(), "results": results})).unwrap());
}
```
