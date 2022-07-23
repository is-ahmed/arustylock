use orion::{aead, aead::SecretKey};
use std::fs::File;
use std::fs::OpenOptions;
use std::io::prelude::*;

pub fn encrypt_data(file: &mut File, key_ref: &aead::SecretKey) {
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .expect("Error reading file to buffer");
    let cipher_text = aead::seal(key_ref, &buffer).unwrap();
    file.write_all(&cipher_text).unwrap();
}

pub fn decrypt_data(file: &mut File, key_ref: &aead::SecretKey) -> Vec<u8> {
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .expect("Error reading file to buffer");
    let decrypted_data = aead::open(key_ref, &buffer).unwrap();
    file.write_all(&decrypted_data).unwrap();
    return decrypted_data;
}
