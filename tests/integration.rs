use std::fs;

use riftx::{pack, unpack};
use tempfile::tempdir;

#[test]
fn test_pack() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("hello.txt"), "Hello, RiftX!").unwrap();

    let archive = dir.path().join("out.riftx");

    let result = pack(&source, &archive, "test_password");
    assert!(result.is_ok(), "packing failed: {:?}", result.err());
    assert!(archive.exists(), "archive file should exist after packing");
    assert!(
        source.join("hello.txt").exists(),
        "original file should still exist after packing"
    );
}

#[test]
fn test_round_trip() {
    let dir = tempdir().unwrap();

    // Build a source tree with a nested file
    let source = dir.path().join("source");
    fs::create_dir_all(source.join("sub")).unwrap();
    fs::write(source.join("root.txt"), "root content").unwrap();
    fs::write(source.join("sub").join("nested.txt"), "nested content").unwrap();

    let archive = dir.path().join("out.riftx");
    let destination = dir.path().join("unpacked");

    pack(&source, &archive, "correct_password").expect("pack should succeed");
    unpack(&archive, &destination, "correct_password").expect("unpack should succeed");

    assert_eq!(
        fs::read_to_string(destination.join("root.txt")).unwrap(),
        "root content"
    );
    assert_eq!(
        fs::read_to_string(destination.join("sub").join("nested.txt")).unwrap(),
        "nested content"
    );
}

#[test]
fn test_wrong_password_is_rejected() {
    let dir = tempdir().unwrap();

    let source = dir.path().join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("secret.txt"), "top secret").unwrap();

    let archive = dir.path().join("out.riftx");
    let destination = dir.path().join("unpacked");

    pack(&source, &archive, "correct_password").expect("pack should succeed");
    let result = unpack(&archive, &destination, "wrong_password");

    assert!(result.is_err(), "unpack with wrong password should fail");
    assert!(
        !destination.exists(),
        "partial extraction should be cleaned up on failure"
    );
}
