pub mod vars;
pub mod ast;
pub mod tokens;

use crate::parser::ast::{build_tree};
use crate::parser::tokens::{tokenize};

pub fn exec(reader: &mut dyn std::io::BufRead, ctx: vars::Context) {
    let tokens = tokenize(reader).unwrap();

    dbg!(&tokens);

    let expressions = build_tree(tokens);

    dbg!(&expressions);
}

pub fn escape(str: String) -> String {
    str
}
