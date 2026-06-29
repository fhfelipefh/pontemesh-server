use sha2::{Digest, Sha256};

pub fn hash_bearer_token(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

pub fn hash_session_token(token: &str) -> String {
    hash_bearer_token(token)
}
