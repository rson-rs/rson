use std::error::Error;

use rson_rs::{de, ser};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
struct Address {
    street: String,
    city: String,
    zip: u64,
}

#[derive(Debug, Default, Deserialize, Serialize)]
enum UserRole {
    Admin,
    #[default]
    User,
    Guest,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
struct User {
    id: usize,
    name: String,
    email: String,
    is_active: bool,
    scores: Vec<i32>,
    address: Address,
    role: UserRole,
}

fn main() -> Result<(), Box<dyn Error>> {
    let data = r#"
    let city = "Rustville";
    let default_address = Address {
        street: "123 Rust St.",
        city,
        zip: 10001
    };
    let user = User {
        id: 1,
        name: "Alice",
        email: "alice@example.com",
        is_active: false,
        scores: [98, 87, 92],
        address: default_address,
        ..
    };

    [
        user,
        User {
            id: 2,
            name: "Bob",
            email: "bob@example.com",
            address: Address {
                street: "456 Rust St.",
                city: user.address.city,
                zip: 10002,
            },
            role: UserRole::Admin,
            ..user
        },
    ]
    "#;

    let deserial = de::builder().from_str(data)?;
    let default_address: Address = deserial.deserialize_var("default_address")?;
    let user: User = deserial.deserialize_var("user")?;
    let users: Vec<User> = deserial.deserialize()?;

    println!("{users:#?}");

    let mut serial = ser::pretty();
    let city_stmt = serial.to_string_var(&default_address.city, "city")?;
    let default_address_stmt = serial
        .add_vars([(["city"], "city")])
        .to_string_var(&default_address, "default_address")?;
    let user_stmt = serial
        .add_vars([(vec!["address"], "default_address"), (vec!["..", "role"], "")])
        .to_string_var(&user, "user")?;
    let users_stmt = serial
        .with_vars([
            (vec!["0"], "user"),
            (vec!["1", "address", "city"], "user.address.city"),
            (vec!["1", "..", "is_active", "scores"], "user"),
        ])
        .to_string(&users)?;

    let output = format!("{city_stmt}\n{default_address_stmt}\n{user_stmt}\n\n{users_stmt}");
    println!("\n{output}");

    Ok(())
}
