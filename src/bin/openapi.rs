//! Writes the OpenAPI document to the given path, or stdout if none is given.

use std::{env, fs};

fn main() {
    let json = bagel::openapi().to_pretty_json().unwrap() + "\n";
    match env::args().nth(1) {
        Some(path) => fs::write(path, json).unwrap(),
        None => print!("{json}"),
    }
}
