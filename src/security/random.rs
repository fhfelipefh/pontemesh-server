use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand_core::{OsRng, RngCore};

pub fn secure_url_token(prefix: &str, bytes: usize) -> String {
    let mut random_bytes = vec![0_u8; bytes];
    OsRng.fill_bytes(&mut random_bytes);
    format!("{prefix}{}", URL_SAFE_NO_PAD.encode(random_bytes))
}
