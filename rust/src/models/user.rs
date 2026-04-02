use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
        pub last_name: String,
        pub email: String,
        pub password: i32,
        pub test: String,
}
