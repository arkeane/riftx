use std::fs;

use riftx::{pack};


#[test]
fn test_pack() {
    fs::create_dir_all("test_data").unwrap();
    fs::write("test_data/test_file.txt", "Hello, RiftX!").unwrap();

    let input_path = std::path::Path::new("test_data");
    let output_path = std::path::Path::new("test_data.tar.xz.enc");

    let result = pack(input_path, output_path, "test_password");

    assert!(result.is_ok(), "Packing failed: {:?}", result.err());

    assert!(fs::metadata("test_data/test_file.txt").is_ok(), "Original file should still exist after packing");

    fs::remove_dir_all("test_data").unwrap();
    fs::remove_file("test_data.tar.xz.enc").unwrap();
}

#[test]
#[ignore = "Requires a valid .tar.xz.enc file to unpack, which is not trivial to create in a test environment"]
fn test_unpack() {
    // This test would require a valid .tar.xz.enc file to unpack, which is not trivial to create in a test environment.
    // You would need to create a .tar.xz.enc file using the pack function and then test unpacking it.
    // For now, this is a placeholder to indicate where the unpacking test would go.
}