// src/models/user.rs
use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub created_at: i64,
}

#[derive(Deserialize, Debug)]
pub struct RegisterForm {
    pub username: String,
    pub password: String,
    pub confirm_password: String,
}

#[derive(Deserialize, Debug)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}
