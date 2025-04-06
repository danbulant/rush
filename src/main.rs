use std::io::Read;
use chumsky::Parser;

pub mod parser;
pub mod executor;

fn main() {
    let mut file = std::fs::File::open("./test/parsetest.rush").expect("Unable to open file");
    let mut string = String::new();
    file.read_to_string(&mut string).unwrap();

    dbg!(&string);

    let parsed = parser::parse().parse(&string);

    println!("{:?}",parsed);

    if parsed.has_errors() {
        println!("Parsing failed");
        for error in parsed.errors() {
            println!("{:?}", error);
        }
        return;
    } else {
        println!("Parsing succeeded");
    }
}