mod parser;

use std::io::{self, Read, Write};
use std::cmp;
use std::convert::TryInto;

use termion::raw::IntoRawMode;
use termion::input::TermRead;
use termion::cursor::{self, DetectCursorPos};
use termion::event::*;
use termion::input::{MouseTerminal};

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout().into_raw_mode().unwrap();
    let ctx = parser::vars::Context {
        scopes: Vec::new(),
        parent_context: None
    };
    let mut output = String::new();
    let mut idx = 0;
    for c in stdin.keys() {
        let c = c.unwrap();
        match c {
            Key::Char('\n') => {
                break;
            }
            Key::Backspace => {
                if output.len() > 0 && idx > 0 {
                    output.remove(idx - 1);
                    idx -= 1;
                    print!("{}{} ", cursor::Left((output.len() - idx + 2).try_into().unwrap()), &output[idx..]);
                }
            }
            Key::Delete => {
                if idx < output.len() {
                    output.remove(idx);
                    print!("{} {}", &output[idx..], cursor::Left((output.len() - idx + 1).try_into().unwrap()));
                }
            }
            Key::End => {
                let start = idx;
                idx = cmp::max(output.len(), 1) - 1;
                print!("{}", cursor::Right((idx - start).try_into().unwrap()));
            }
            Key::Home => {
                let start = idx;
                idx = 0;
                print!("{}", cursor::Left(start.try_into().unwrap()));
            }
            Key::Left => {
                if idx > 0 {
                    idx -= 1;
                }
                print!("{}", cursor::Left(1));
            }
            Key::Right => {
                if idx < output.len() - 1 {
                    idx += 1;
                }
                print!("{}", cursor::Right(1));
            }
            Key::Char(char) => {
                output.insert(idx, char);
                idx += 1;
                print!("{}", char);
            }
            _ => {}
        }
        stdout.flush().unwrap();
    }
    parser::read_parse(&mut output.as_bytes(), ctx).unwrap();
}
