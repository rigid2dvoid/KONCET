//! Architecture guard: the simulation engine is deterministic and local. It may
//! use threads only through the app ticker, never network/process/filesystem I/O.

use std::fs;
use std::path::Path;

const FORBIDDEN: &[&str] = &[
    "std::fs",
    "std::net",
    "std::process",
    "Command::new",
    "TcpStream",
    "TcpListener",
    "UdpSocket",
    "reqwest",
    "hyper::",
    "tokio::net",
];

#[test]
fn simulation_source_has_no_external_io() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut checked = 0;
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        checked += 1;
        let source = fs::read_to_string(&path).unwrap();
        for token in FORBIDDEN {
            assert!(
                !source.contains(token),
                "{} contains forbidden I/O token {token}",
                path.display()
            );
        }
    }
    assert!(checked > 0, "architecture scan found no modules");
}
