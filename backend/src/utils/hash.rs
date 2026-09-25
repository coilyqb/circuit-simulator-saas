use crate::errors::AppError;

pub fn hash_password(password: &str, cost: u32) -> Result<String, AppError> {
    bcrypt::hash(password, cost).map_err(AppError::from)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    bcrypt::verify(password, hash).map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use super::{hash_password, verify_password};

    #[test]
    fn password_hash_round_trip() {
        let password = "Sup3rSecure!";
        let hash = hash_password(password, 4).expect("hash");

        assert_ne!(password, hash);
        assert!(verify_password(password, &hash).expect("verify"));
        assert!(!verify_password("wrong-password", &hash).expect("verify wrong"));
    }
}
