pub mod vars;
pub mod ast;
pub mod tokens;

use std::io;
use std::io::Read;
//use std::io::prelude::*;
use utf8_chars::BufReadCharsExt;
use crate::parser::ast::{build_tree, Expression};
use crate::parser::tokens::{tokenize, Tokens};

pub fn exec(reader: &mut dyn std::io::BufRead, ctx: vars::Context) {
    let tokens = tokenize(reader).unwrap();

    dbg!(&tokens);

    println!("Building tree");
    let expressions = build_tree(tokens);

    dbg!(&expressions);
}

pub fn escape(str: String) -> String {
    str
}