use std::fs::File;
use std::process::{Child, Command, Stdio};
use std::io;
use crate::parser::ast::{AndExpression, CommandValue, Expression, FileSourceExpression, FileTargetExpression, LetExpression, OrExpression, RedirectTargetExpression, Value};
use crate::parser::vars;
use crate::parser::vars::{Context, Variable};

trait ExecExpression {
    fn exec(self, ctx: &mut vars::Context) -> Option<Command>;
}

trait GetValue {
    fn get(self, ctx: &mut vars::Context) -> Variable;
}

struct ExecResult {
    cmd: Option<Command>,
    child: Option<Child>
}

impl ExecResult {
    fn new(cmd: Option<Command>, child: Option<Child>) -> Self {
        Self { cmd, child }
    }
    /// Spawns the result, running the command (if any). Non-command results won't be spawned (like let statements)
    fn spawn(&mut self) -> &mut Self {
        if !self.started() {
            match &mut self.cmd {
                None => {},
                Some(cmd) => {
                    self.child = Some(cmd.spawn().unwrap());
                }
            }
        }
        self
    }
    /// Checks if the result was spawned before by checking the child property. Non-command results won't ever be spawned (like let statements)
    fn started(&self) -> bool {
        matches!(self.child, Some(_))
    }
    /// A simple wrapper for redirecting current result (self) into STDIO (files or streams).
    ///
    /// Does spawn the current result
    fn redirect_into<T: std::io::Write>(mut self, into: &mut T) -> &mut T {
        match &mut self.cmd {
            None => {},
            Some(cmd) => {
                cmd.stdout(Stdio::piped());
                self.spawn();
                let child = self.child.unwrap();
                let mut stdout = child.stdout.unwrap();
                io::copy(&mut stdout, into);
            }
        }
        into
    }
    /// A shorthand for redirecting current result into the next one
    ///
    /// Uses `redirect_from_result` of the next result. Spawns this result, but not the next one.
    fn redirect_into_result(&mut self, into: &mut ExecResult) -> &mut Self {
        into.redirect_from_result(self);
        self
    }
    /// Redirects the `from` into the current pending result
    ///
    /// Doesn't spawn the current result
    fn redirect_from<T: Into<Stdio>>(&mut self, from: T) -> &mut Self {
        match &mut self.cmd {
            None => {},
            Some(cmd) => {
                cmd.stdin(from);
            }
        }
        self
    }
    /// A shortcut for redirecting a previous result into the current one
    ///
    /// Spawns the previous result to obtain the output, but not the current one (self)
    fn redirect_from_result(&mut self, into: &mut ExecResult) -> io::Result<&mut Self> {
        if matches!(self.cmd, None) {
            return Ok(self);
        }
        match &mut self.cmd {
            None => {},
            Some(source) => {
                source.stdout(Stdio::piped());
                match &mut into.cmd {
                    None => {},
                    Some(target) => {
                        target.stdin(source.spawn()?.stdout.unwrap());
                    }
                };
            }
        }
        Ok(self)
    }
}

impl GetValue for CommandValue {
    fn get(self, ctx: &mut vars::Context) -> Variable {
        match self {
            CommandValue::Value(val) => val.get(ctx),
            CommandValue::Var(_, _) => panic!("Broken executor")
        }
    }
}

impl GetValue for Value {
    fn get(self, ctx: &mut vars::Context) -> Variable {
        match self {
            Value::Literal(str) => {
                Variable::String(str)
            },
            Value::Variable(str) => ctx.get_var(&str).unwrap_or(&mut Variable::String(String::from(""))).clone(),
            Value::ArrayVariable(str) => ctx.get_var(&str).unwrap_or(&mut Variable::Array(Vec::new())).clone(),
            Value::ArrayFunction(_) => panic!("Not implemented yet"),
            Value::StringFunction(_) => panic!("Not implemented yet"),
            Value::Expressions(expressions) => {
                let mut out = String::new();
                ctx.add_scope(true);
                for mut expr in expressions {
                    let res = expr.exec(ctx);
                    match res {
                        None => {},
                        Some(mut cmd) => {
                            out += &*String::from_utf8_lossy(&cmd.output().expect("Failed to read output of command").stdout);
                        }
                    }
                }
                ctx.pop_scope();
                Variable::String(out)
            },
            Value::Values(vec) => {
                let mut out = Vec::new();
                for mut val in vec {
                    out.push(val.get(ctx));
                }
                Variable::Array(out)
            }
        }
    }
}

impl ExecExpression for Expression {
    fn exec(self, ctx: &mut vars::Context) -> Option<Command> {
        match self {
            Expression::LetExpression(expr) => expr.exec(ctx),
            Expression::Command(expr) => expr.exec(ctx),
            Expression::JobCommand(_) => todo!(),
            Expression::Function(_) => todo!(),
            Expression::IfExpression(_) => todo!(),
            Expression::WhileExpression(_) => todo!(),
            Expression::ForExpression(_) => todo!(),
            Expression::RedirectTargetExpression(expr) => expr.exec(ctx),
            Expression::FileTargetExpression(expr) => expr.exec(ctx),
            Expression::FileSourceExpression(expr) => expr.exec(ctx),
            Expression::Expressions(expr) => expr.exec(ctx),
            Expression::OrExpression(expr) => expr.exec(ctx),
            Expression::AndExpression(expr) => expr.exec(ctx)
        }
    }
}

impl ExecExpression for LetExpression {
    fn exec(mut self, ctx: &mut vars::Context) -> Option<Command> {
        let key = self.key.get(ctx);
        let val = self.value.get(ctx);
        ctx.set_var(key.to_string(), val);
        None
    }
}

impl ExecExpression for Vec<CommandValue> {
    fn exec(mut self, ctx: &mut vars::Context) -> Option<Command> {
        if self.len() == 0 { panic!("Command with 0 length"); }
        let mut first = self.remove(0);
        let command_name = first.get(ctx).to_string();
        let mut cmd = Command::new(command_name);
        for mut value in self {
            cmd.arg(value.get(ctx).to_string());
        }
        Some(cmd)
    }
}

impl ExecExpression for RedirectTargetExpression {
    fn exec(mut self, ctx: &mut vars::Context) -> Option<Command> {
        let mut src = self.source.exec(ctx).unwrap();
        let mut target = self.target.exec(ctx).unwrap();
        src.stdout(Stdio::piped());
        match src.spawn() {
            Result::Err(e) => { println!("Error executing: {}", e)},
            Result::Ok(mut res) => {
                target.stdin(res.stdout.unwrap());
            }
        }

        Some(target)
    }
}

impl ExecExpression for FileTargetExpression {
    fn exec(mut self, ctx: &mut vars::Context) -> Option<Command> {
        let mut src = self.source;
        let mut target = self.target.get(ctx);
        let mut src = match src {
            Some(expr) => expr.exec(ctx),
            None => {
                todo!();
            }
        };
        let command;
        match src {
            Some(mut cmd) => {
                cmd.stdout(Stdio::piped());
                let mut file = File::create(target.to_string());
                match file {
                    Result::Err(e) => println!("Error: {}", e),
                    Result::Ok(mut file) => {
                        match cmd.spawn() {
                            Result::Err(e) => {
                                println!("Error executing command: {}", e);
                            },
                            Result::Ok(res) => {
                                io::copy(&mut res.stdout.unwrap(), &mut file);
                            }
                        }
                    }
                }
                command = cmd;
            },
            None => { panic!("Invalid command provided for file target"); }
        };
        Some(command)
    }
}

impl ExecExpression for FileSourceExpression {
    fn exec(self, ctx: &mut Context) -> Option<Command> {
        let mut source = self.source.get(ctx).to_string();
        let mut target = self.target;
        let mut target = match target {
            Some(expr) => expr.exec(ctx),
            None => {
                Some(Command::new("less"))
            }
        };
        let mut command = match target {
            None => { panic!("Invalid command") },
            Some(cmd) => cmd
        };
        let mut source = match File::open(source) {
            Result::Err(e) => panic!("Cannot open file: {}", e),
            Result::Ok(file) => file
        };
        command.stdin(source);

        Some(command)
    }
}

impl ExecExpression for Vec<Expression> {
    fn exec(self, ctx: &mut Context) -> Option<Command> {
        let mut last = None;
        for expr in self {
            last = expr.exec(ctx);
        }
        last
    }
}

impl ExecExpression for OrExpression {
    fn exec(self, ctx: &mut Context) -> Option<Command> {
        let mut first = match self.first.exec(ctx) {
            None => panic!("Invalid OR expression"),
            Some(cmd) => cmd
        };
        let mut res = match first.spawn() {
            Result::Err(e) => {
                self.second.exec(ctx)
            },
            Result::Ok(mut res) => {
                if res.wait().unwrap().success() {
                    Some(first)
                } else {
                    self.second.exec(ctx)
                }
            }
        };

        res
    }
}

impl ExecExpression for AndExpression {
    fn exec(self, ctx: &mut Context) -> Option<Command> {
        let mut first = match self.first.exec(ctx) {
            None => panic!("Invalid AND expression"),
            Some(cmd) => cmd
        };
        let mut res = match first.spawn() {
            Result::Err(e) => {
                Some(first)
            },
            Result::Ok(mut res) => {
                if !res.wait().unwrap().success() {
                    Some(first)
                } else {
                    self.second.exec(ctx)
                }
            }
        };

        res
    }
}

pub fn exec_tree(tree: Vec<Expression>, ctx: &mut vars::Context) {
    for mut expression in tree {
        let mut cmd = expression.exec(ctx);
        match cmd {
            None => {},
            Some(mut cmd) => match cmd.spawn() {
                Result::Err(e) => {
                    println!("Error executing: {}", e);
                },
                Result::Ok(mut res) => {
                    let out = res.wait().unwrap();
                    ctx.set_var(String::from("?"), Variable::I32(out.code().unwrap_or(1)));
                }
            }
        }
    }
}
