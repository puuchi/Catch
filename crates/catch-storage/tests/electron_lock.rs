#![cfg(windows)]
use catch_storage::ProfileLock;
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

#[test]
fn node_and_rust_cannot_write_the_same_profile_at_the_same_time() {
    let dir = tempfile::tempdir().unwrap();
    let script = r#"const {ProfileLock}=require(process.argv[1]);const lock=new ProfileLock(process.argv[2]);console.log('LOCKED');process.stdin.resume();process.stdin.once('data',()=>{lock.close();process.exit(0)})"#;
    let adapter = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../src/infrastructure/persistence/profile-lock.js");
    let mut node = Command::new("node")
        .args(["-e", script])
        .arg(&adapter)
        .arg(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut ready = String::new();
    BufReader::new(node.stdout.take().unwrap())
        .read_line(&mut ready)
        .unwrap();
    assert_eq!(ready.trim(), "LOCKED");
    assert!(ProfileLock::acquire(dir.path()).is_err());
    use std::io::Write;
    node.stdin.as_mut().unwrap().write_all(b"exit\n").unwrap();
    assert!(node.wait().unwrap().success());
    let rust = ProfileLock::acquire(dir.path()).unwrap();
    let output=Command::new("node").args(["-e","try{new(require(process.argv[1]).ProfileLock)(process.argv[2]);process.exit(1)}catch{process.exit(0)}"]).arg(adapter).arg(dir.path()).output().unwrap();
    assert!(output.status.success());
    drop(rust);
    let output = Command::new("node")
        .args([
            "-e",
            "new(require(process.argv[1]).ProfileLock)(process.argv[2]).close()",
        ])
        .arg(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../src/infrastructure/persistence/profile-lock.js"),
        )
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
}
