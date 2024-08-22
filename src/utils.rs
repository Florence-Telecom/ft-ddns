use std::sync::Mutex;

use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand::{
    distributions::{Alphanumeric, DistString},
    rngs::StdRng,
    SeedableRng,
};
use rocket::{fairing::AdHoc, serde::Deserialize};

/// Hash parameter must be a valid password hash.
pub fn compare_with_hash(password: &str, hash: &str) -> Result<(), argon2::password_hash::Error> {
    let algs: &[&dyn PasswordVerifier] = &[&Argon2::default()];
    let password_hash: PasswordHash = PasswordHash::new(hash).unwrap();
    password_hash.verify_password(algs, password)
}

pub fn generate_random_password(rng: &Mutex<StdRng>) -> (String, String) {
    let mut lock = rng.lock().unwrap();
    let password = Alphanumeric.sample_string(&mut *lock, 24);

    let salt = SaltString::generate(&mut *lock);
    let argon2 = Argon2::default();
    let password_hash: PasswordHash = argon2.hash_password(password.as_bytes(), &salt).unwrap();

    (password, password_hash.to_string())
}

pub fn stage_rng() -> AdHoc {
    AdHoc::on_ignite("Cryptographically secure RNG", |rocket| {
        Box::pin(async {
            let rng: StdRng = StdRng::from_entropy();

            rocket.manage(Mutex::new(rng))
        })
    })
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Credentials {
    pub(crate) username: String,
    pub(crate) password: String,
}
