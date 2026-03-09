pub mod errors;
pub mod lmdb;

/// Simple utility function to convert a string to u128 using a basic polynomial rolling hash algorithm.
/// Mostly used for generating keys from hard-coded strings
pub fn string_to_u128(s: &str) -> u128 {
    s.as_bytes().iter().fold(0, |acc, &b| acc * 31 + b as u128)
}