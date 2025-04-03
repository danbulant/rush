use std::io::Read;
use chumsky::Parser;

mod parser;

fn main() {
    let mut file = std::fs::File::open("./test/parsetest.rush").expect("Unable to open file");
    let mut string = String::new();
    file.read_to_string(&mut string).unwrap();

    dbg!(&string);

    println!("{:?}", parser::parse().parse(&string));
}