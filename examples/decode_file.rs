extern crate rson_rs as rson;
#[macro_use]
extern crate serde_derive;

use std::collections::HashMap;
use std::fs::File;

use rson::de::from_reader;

#[derive(Debug, Deserialize)]
struct Config
{
    boolean: bool,
    float: f32,
    map: HashMap<u8, char>,
    nested: Nested,
    tuple: (u32, u32),
}

#[derive(Debug, Deserialize)]
struct Nested
{
    a: String,
    b: char,
}

fn main()
{
    let input_path = format!("{}/examples/example.rson",
                             env!("CARGO_MANIFEST_DIR"));
    let f = File::open(&input_path).expect("Failed opening file");
    let config: Config = match from_reader(f) {
        Ok(x) => x,
        Err(e) => {
            println!("Failed to load config: {}", e);

            ::std::process::exit(1);
        },
    };

    println!("Config: {:?}", &config);
}
