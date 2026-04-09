mod engine;

use engine::archive::{compress_directory, decompress_file};
use engine::crypto::{encrypt, decrypt, generate_key};

pub fn pack(input: &str) {
    println!("Packing : {}", input);
    let data =compress_directory(input);
    encrypt(data, generate_key().as_slice());
}

pub fn unpack(input: &str) {
    println!("Unpacking : {}", input);
    decompress_file(input);
    decrypt(input.as_bytes(), generate_key().as_slice());
}