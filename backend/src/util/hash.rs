use constant_time_eq::constant_time_eq;
use sha2::{Digest, Sha256};

pub fn hash(data: &str) -> String {
    let mut hasher = Sha256::new();
    let salt = dotenvy::var("HASH_SECRET").expect("HASH_SECRET must be set in .env file");
    hasher.update(salt);
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

pub fn validate(original: &str, hashed: &str) -> bool {
    let original_hashed = hash(original);
    constant_time_eq(original_hashed.as_bytes(), hashed.as_bytes())
}
