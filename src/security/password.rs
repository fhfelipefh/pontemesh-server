use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};

pub fn validate_admin_password(password: &str) -> anyhow::Result<()> {
    if password.chars().count() < 12 {
        anyhow::bail!("password must have at least 12 characters");
    }
    if !password.chars().any(|character| character.is_lowercase())
        || !password.chars().any(|character| character.is_uppercase())
        || !password.chars().any(|character| character.is_numeric())
        || !password
            .chars()
            .any(|character| !character.is_alphanumeric())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_strong_ascii_and_unicode_admin_passwords() {
        assert!(validate_admin_password("PonteMesh123!").is_ok());
        assert!(validate_admin_password("ÁrvoreSegura１２!x").is_ok());
    }

    #[test]
    fn rejects_passwords_missing_any_required_character_class() {
        assert!(validate_admin_password("pontemesh123!").is_err());
        assert!(validate_admin_password("PONTEMESH123!").is_err());
        assert!(validate_admin_password("PonteMeshAdmin!").is_err());
        assert!(validate_admin_password("PonteMesh1234").is_err());
        assert!(validate_admin_password("Short1!").is_err());
    }

    #[test]
    fn hash_and_verify_admin_password_lifecycle() {
        let raw = "PonteMesh123!";
        let hash = hash_admin_password(raw).expect("hash success");
        assert!(verify_admin_password(raw, &hash).expect("verify ok"));
        assert!(!verify_admin_password("WrongPassword1!", &hash).expect("verify fail"));
        assert!(verify_admin_password(raw, "invalid_hash_string").is_err());
    }
}
