pub mod vars;

use std::io;
//use std::io::prelude::*;
use utf8_chars::BufReadCharsExt;

pub fn read_parse(reader: &mut dyn std::io::BufRead, ctx: vars::Context) -> io::Result<()> {
    for c in reader.chars().map(|x| x.unwrap()) {
        println!("char {}\r", c);
    }
    Ok(())
}
