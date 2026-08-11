use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};

pub fn validate_admin_password(password: &str) -> anyhow::Result<()> {
    if password.len() < 12 {
        anyhow::bail!("password must have at least 12 characters");
    }
    if !password
        .chars()
        .any(|character| character.is_ascii_lowercase())
        || !password
            .chars()
            .any(|character| character.is_ascii_uppercase())
        || !password.chars().any(|character| character.is_ascii_digit())
        || !password
            .chars()
            .any(|character| !character.is_ascii_alphanumeric())
    {
        anyhow::bail!("password must include upper, lower, number, and symbol");
    }
    Ok(())
}

pub fn hash_admin_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|error| anyhow::anyhow!("failed to hash admin password with Argon2id: {error}"))?;
    Ok(hash.to_string())
}

pub fn verify_admin_password(password: &str, password_hash: &str) -> anyhow::Result<bool> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|error| anyhow::anyhow!("failed to parse admin password hash: {error}"))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
