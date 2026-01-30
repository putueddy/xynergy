// Temporary utility to hash password
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use rand::thread_rng;

fn main() {
    let password = "admin123";
    let salt = SaltString::generate(&mut thread_rng());
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    println!("Hashed password: {}", password_hash);
}
