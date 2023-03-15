use orion::aead;
use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;


 


pub fn reset_file_cursor(file: &mut File) {
    file.seek(SeekFrom::Start(0)).expect("Failed to seek");
}

// TODO: Make sure these functions return results
pub fn encrypt_data(file: &mut File, key_ref: &aead::SecretKey) {
    let mut buffer = Vec::new();
    reset_file_cursor(file);
    file.read_to_end(&mut buffer)
        .expect("Error reading file to buffer");
    let cipher_text = aead::seal(key_ref, &buffer).unwrap();
    reset_file_cursor(file);
    file.set_len(0).unwrap();
    file.write_all(&cipher_text).unwrap();
    reset_file_cursor(file);
}

/*
*/
pub fn decrypt_data(file: &mut File, key_ref: &aead::SecretKey) -> Vec<u8> {
    let mut buffer = Vec::new();
    reset_file_cursor(file);
    file.read_to_end(&mut buffer)
        .expect("Error reading file to buffer");
    let decrypted_data = aead::open(key_ref, &buffer).unwrap();
    reset_file_cursor(file);
    return decrypted_data;
}
