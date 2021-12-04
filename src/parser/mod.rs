pub mod vars;
pub mod ast;

use std::io;
//use std::io::prelude::*;
use utf8_chars::BufReadCharsExt;

pub fn exec(reader: &mut dyn std::io::BufRead, ctx: vars::Context) -> io::Result<()> {
    println!("got result");
    let mut word = String::new();
    let mut if_active = false;
    let mut quote_active = false;
    let mut double_quote_active = false;
    let mut var_active = false;
    let mut var_parens_active = false;
    let mut command = String::new();
    let mut args: Vec<String> = Vec::new();
    let mut var_name = String::new();
    let mut prev_char: Option<char> = None;

    for c in reader.chars().map(|x| x.unwrap()) {
        print!("{}\r", c);
        // loop runs once, allows early exit
        loop {
            match c {
                '$' => {
                    if quote_active {
                        word.push('$');
                    }
                    var_active = true;
                }
                '{' => {
                    if var_active {
                        var_parens_active = true;
                    } else {
                        word.push('{');
                    }
                }
                '}' => {
                    if !var_parens_active {
                        word.push('}');
                    }
                }
                ' ' => {
                    if quote_active || double_quote_active {
                        word.push(' ');
                        break;
                    }
                    if var_parens_active { break; }
                    if var_active {}
                    if command.len() == 0 {
                        if word == "if" {
                            if_active = true;
                        } else if word == "end" {
                            ctx.pop_scope();
                        } else {
                            command = word;
                            word = String::new();
                        }
                    } else {
                        args.push(word.clone());
                        word = String::new();
                    }
                }
                ';' | '\n' => {
                    let mut arg_strings: Vec<&str> = Vec::new();
                    for arg in &args {
                        arg_strings.push(&*arg);
                    }
                    println!("Will execute {} with {}", command, arg_strings.join(" "));
                    if if_active {
                        println!("Will execute below only if the previous command succeeded");
                    }
                    if_active = false;

                    let exit_code = 0;
                    ctx.add_scope(exit_code == 0);
                }
                char => {
                    if var_active {
                        var_name.push(char);
                    } else {
                        word.push(char);
                    }
                }
            };
            break;
        }
        prev_char = Some(c);
    }
    Ok(())
}

pub fn escape(str: String) -> String {
    str
}