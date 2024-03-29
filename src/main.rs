mod parser;
mod env;
mod nativeFunctions;

use std::io::{self, BufRead, Stdout, Write};
use std::cmp;
use std::convert::TryInto;
use std::path::Path;
use std::process;
use clap::{Command, arg};
use termion::raw::{IntoRawMode, RawTerminal};
use termion::input::TermRead;
use termion::cursor::{DetectCursorPos};
use termion::event::*;
use std::fs::File;
use std::io::BufReader;
use anyhow::Result;
use crate::nativeFunctions::get_native_functions;
use crate::parser::vars::Variable;

struct Term {
    input: String,
    idx: usize,
}

impl Term {
    fn new() -> Term {
        Term {
            input: String::new(),
            idx: 0,
        }
    }

    fn print(self: &Self, stdout: &mut RawTerminal<Stdout>) {
        print!("{}", self.format(stdout));
    }
    fn format(self: &Self, stdout: &mut RawTerminal<Stdout>) -> String {
        let (_, y) = stdout.cursor_pos().unwrap();
        format!(
            "{}{}{}{}{}{}",
            termion::clear::CurrentLine,
            termion::cursor::Goto(1, y),
            "$: ",
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
        Shell {
            term: Term::new(),
            ctx: parser::vars::Context::new()
        }
    }

    fn collect(&mut self) {
        let stdin = std::io::stdin();
        let v = stdin.lock().lines().next().unwrap().unwrap();
        self.term.input = v;
    }


    fn edit(&mut self) {
        let stdin = io::stdin();
        let mut stdout = io::stdout().into_raw_mode().unwrap();
        for c in stdin.keys() {
            let c = c.unwrap();
            match c {
                Key::Char('\n') => {
                    if self.term.input.chars().nth(self.term.idx).unwrap_or(' ') == '\\' {
                        self.term.insert_str(self.term.idx, "\\\n");
                    } else {
                        break;
                    }
                }
                Key::Backspace => {
                    if !self.term.input.is_empty() && self.term.idx > 0 {
                        if self.term.idx == self.term.input.len() - 1 {
                            self.term.input.pop();
                        } else {
                            self.term.remove(self.term.idx - 1);
                        }
                        self.term.idx -= 1;
                    }
                }
                Key::Delete => {
                    if self.term.idx < self.term.input.len() {
                        self.term.remove(self.term.idx);
                    }
                }
                Key::End => {
                    self.term.idx = cmp::max(self.term.input.len(), 1) - 1;
                }
                Key::Home => {
                    self.term.idx = 0;
                }
                Key::Left => {
                    if self.term.idx > 0 {
                        self.term.idx -= 1;
                    }
                }
                Key::Right => {
                    if self.term.idx < self.term.input.len() - 1 {
                        self.term.idx += 1;
                    }
                }
                Key::Ctrl('c') => {
                    process::exit(1);
                }
                Key::Ctrl('d') => {
                    process::exit(0);
                }
                Key::Char(char) => {
                    self.term.insert(self.term.idx, char);
                    self.term.idx += 1;
                }
                _ => {}
            }
            self.term.print(&mut stdout);
            stdout.flush().unwrap();
        }
        stdout.suspend_raw_mode().unwrap();
    }

    fn start() {
        let mut shell = Shell::new();
        shell.ctx.native_func = get_native_functions();
        loop {
            print!("$: ");
            io::stdout().flush().unwrap();
            shell.collect();
            if shell.term.input == "exit" {
                break;
            }
            shell.term.input += "\n";
            shell.ctx.exports = env::os_env_hashmap().into_iter().map(|(k, v)| (k, Variable::String(v))).collect();
            let res = parser::exec(&mut shell.term.input.as_bytes(), &mut shell.ctx);
            if let Err(err) = res { eprintln!("rush: {}", err) }
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

    if let Some(command) = matches.value_of("command") {
        let mut ctx = parser::vars::Context::new();
        parser::exec(&mut command.as_bytes(), &mut ctx).unwrap();
        return;
    };
    if let Some(file) = matches.value_of("file") {
        load_and_run(file).unwrap();
        return;
    };
    Shell::start();
}

#[cfg(test)]
mod test {
    use crate::{load_and_run};
    use anyhow::Result;
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
