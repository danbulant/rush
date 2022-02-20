pub mod vars;
pub mod ast;
pub mod tokens;
mod exec;

use crate::parser::ast::{build_tree};
use crate::parser::exec::exec_tree;
use crate::parser::tokens::{tokenize};
use anyhow::Result;

pub fn exec(reader: &mut dyn std::io::BufRead, ctx: &mut vars::Context) -> Result<()> {
    let tokens = tokenize(reader).unwrap();

    dbg!(&tokens);

    let expressions = build_tree(tokens);

    dbg!(&expressions);

    exec_tree(expressions?, ctx);
    Ok(())
}

pub fn escape(str: String) -> String {
    str
}
