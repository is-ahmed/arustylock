use arustylock::encryption::encryption::*;
use orion::{aead, aead::SecretKey};
use std::fs::File;
const SAMPLE_FILE_PATHS: [&str; 2] = ["./sample1.json", "./sample2.json"];

fn test_encrypt_sanity() {
    let secret_key = SecretKey::default();
    for i in 0..SAMPLE_FILE_PATHS.len() {
        let mut file = File::open(SAMPLE_FILE_PATHS[i]).unwrap();
        encrypt_data(&mut file, &secret_key);
    }
}

fn test_decrypt_sanity() {
    let secret_key = SecretKey::default();

    for i in 0..SAMPLE_FILE_PATHS.len() {
        let mut file = File::open(SAMPLE_FILE_PATHS[i]).unwrap();
        decrypt_data(&mut file, &secret_key)
    }
}
#[test]
fn it_works() {
    let result = 2 + 2;
    assert_eq!(result, 4);
}
