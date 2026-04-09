pub fn encrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    println!("Encrypting data with key: {:?}", key);    // should not print the key in a real implementation
    data.to_vec()                                       // Placeholder, should return encrypted data
}

pub fn decrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    println!("Decrypting data with key: {:?}", key);    // should not print the key in a real implementation
    data.to_vec()                                       // Placeholder, should return decrypted data
}

pub fn generate_key() ->Vec<u8> {
    println!("Generating encryption key");
    let key = "my_secret_key";                    // Placeholder, should be a securely generated key
    key.as_bytes().to_vec()
}