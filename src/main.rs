mod parser;

use std::io::{self, BufRead, Stdout, Write};
use std::cmp;
use std::convert::TryInto;
use std::path::Path;
use std::process;
use clap::{Arg, Command, arg};
use termion::raw::{IntoRawMode, RawTerminal};
use termion::input::TermRead;
use termion::cursor::{DetectCursorPos};
use termion::event::*;
use std::fs::File;
use std::io::BufReader;
use anyhow::Result;

struct Term {
    input: String,
    idx: usize,
}

impl Term {
    fn new() -> Term {
        return Term {
            input: String::new(),
            idx: 0,
        };
    }

    fn print(self: &Self, stdout: &mut RawTerminal<Stdout>) {
        print!("{}", self.format(stdout));
    }
    fn format(self: &Self, stdout: &mut RawTerminal<Stdout>) -> String {
        let (_, y) = stdout.cursor_pos().unwrap();
        format!(
            "{}{}{}{}{}",
            termion::clear::CurrentLine,
            termion::cursor::Goto(1, y),
            &self.input,
            termion::cursor::Left((self.input.len() - self.idx).try_into().unwrap()),
            termion::cursor::Right(if self.input.len() > 0 { 1 } else { 0 } )
        )
    }

    fn insert(self: &mut Self, idx: usize, char: char) {
        self.input.insert(idx, char);
    }
    fn insert_str(self: &mut Self, idx: usize, str: &str) {
        self.input.insert_str(idx, str);
    }
    fn remove(self: &mut Self, idx: usize) {
        self.input.remove(idx);
    }
}

struct Shell {
    term: Term,
    ctx: parser::vars::Context,
}

impl Shell {
    fn new() -> Shell {
        return Shell {
            term: Term::new(),
            ctx: parser::vars::Context::new()
        };
    }

    fn collect(&mut self) {
        let stdin = std::io::stdin();
        let v = stdin.lock().lines().next().unwrap().unwrap();
        self.term.input = v;
    }
}

fn start_shell() {
    let mut shell = Shell::new();
    loop {
        print!("$: ");
        io::stdout().flush().unwrap();
        shell.collect();
        shell.term.input += "\n";
        let res = parser::exec(&mut shell.term.input.as_bytes(), &mut shell.ctx);
        match res {
            Err(err) => eprintln!("rush: {}", err),
            Ok(_) => {}
        }
    }
}

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

fn load_and_run<P: AsRef<Path>>(path: P) -> Result<()> {
    let mut ctx = parser::vars::Context::new();
    let src = File::open(path).unwrap();
    parser::exec(&mut BufReader::new(src), &mut ctx)
}

fn main() {
    let matches = Command::new("Rush")
        .version(VERSION)
        .author(AUTHORS)
        .about(DESCRIPTION)
        .arg(
            arg!([file] "File to execute")
        )
        .arg(
            arg!(-c --command <COMMAND> "Command to execute")
                .required(false)
        )
        .get_matches();

    match matches.value_of("command") {
        Some(command) => {
            let mut ctx = parser::vars::Context::new();
            parser::exec(&mut command.as_bytes(), &mut ctx).unwrap();
            return;
        },
        None => {}
    };
    match matches.value_of("file") {
        Some(file) => {
            load_and_run(file).unwrap();
            return;
        },
        None => {}
    };
    start_shell();
}

#[cfg(test)]
mod test {
    use std::fs::File;
    use std::io::BufReader;
    use std::path::Path;
    use crate::parser;
    use anyhow::Result;

    fn load_and_run<P: AsRef<Path>>(path: P) -> Result<()> {
        let mut ctx = parser::vars::Context::new();
        let src = File::open(path).unwrap();
        parser::exec(&mut BufReader::new(src), &mut ctx)
    }

    #[test]
    fn simple() -> Result<()> {
        load_and_run("test/simple.rush")
    }

    #[test]
    fn var() -> Result<()> {
        load_and_run("test/var.rush")
    }

    #[test]
    fn if_base() -> Result<()> {
        load_and_run("test/base_if.rush")
    }

    #[test]
    fn if_else() -> Result<()> {
        load_and_run("test/if_else.rush")
    }

    #[test]
    fn while_expr() -> Result<()> {
        load_and_run("test/while.rush")
    }
}

fn editor() -> Shell {
    let stdin = io::stdin();
    let mut stdout = io::stdout().into_raw_mode().unwrap();
    let mut shell = Shell::new();
    for c in stdin.keys() {
        let c = c.unwrap();
        match c {
            Key::Char('\n') => {
                if shell.term.input.chars().nth(shell.term.idx).unwrap_or(' ') == '\\' {
                    shell.term.insert_str(shell.term.idx, "\\\n");
                } else {
                    break;
                }
            }
            Key::Backspace => {
                if shell.term.input.len() > 0 && shell.term.idx > 0 {
                    if shell.term.idx == shell.term.input.len() - 1 {
                        shell.term.input.pop();
                    } else {
                        shell.term.remove(shell.term.idx - 1);
                    }
                    shell.term.idx -= 1;
                }
            }
            Key::Delete => {
                if shell.term.idx < shell.term.input.len() {
                    shell.term.remove(shell.term.idx);
                }
            }
            Key::End => {
                shell.term.idx = cmp::max(shell.term.input.len(), 1) - 1;
            }
            Key::Home => {
                shell.term.idx = 0;
            }
            Key::Left => {
                if shell.term.idx > 0 {
                    shell.term.idx -= 1;
                }
            }
            Key::Right => {
                if shell.term.idx < shell.term.input.len() - 1 {
                    shell.term.idx += 1;
                }
            }
            Key::Ctrl('c') => {
                process::exit(1);
            }
            Key::Char(char) => {
                shell.term.insert(shell.term.idx, char);
                shell.term.idx += 1;
            }
            _ => {}
        }
        shell.term.print(&mut stdout);
        stdout.flush().unwrap();
    }
    stdout.suspend_raw_mode().unwrap();
    shell
}
