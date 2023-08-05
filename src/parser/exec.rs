use std::fs::File;
use std::process::{Child, Command, Stdio};
use std::io;
use crate::parser::ast::{AndExpression, BreakExpression, CommandValue, Expression, FileSourceExpression, FileTargetExpression, IfExpression, LetExpression, OrExpression, RedirectTargetExpression, Value, WhileExpression};
use crate::parser::vars;
use crate::parser::vars::{AnyFunction, Context, Variable};
use anyhow::{Result, bail, Context as AnyhowContext};

trait ExecExpression {
    fn exec(&mut self, ctx: &mut vars::Context) -> Result<Option<Command>>;
}

trait GetValue {
    fn get(&mut self, ctx: &mut vars::Context) -> Result<Variable>;
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
    fn get(self: &mut CommandValue, ctx: &mut vars::Context) -> Result<Variable> {
        match self {
            CommandValue::Value(val) => val.get(ctx),
            CommandValue::Var(_, _) => bail!("Broken executor")
        }
    }
}

impl GetValue for Value {
    fn get(self: &mut Value, ctx: &mut vars::Context) -> Result<Variable> {
        match self {
            Value::Literal(str) => {
                Ok(Variable::String(str.clone()))
            },
            Value::Variable(str) => Ok(ctx.get_var(str).unwrap_or(&mut Variable::String(String::from(""))).clone()),
            Value::ArrayVariable(str) => Ok(ctx.get_var(str).unwrap_or(&mut Variable::Array(Vec::new())).clone()),
            Value::Expressions(expressions) => {
                let mut out = String::new();
                ctx.add_scope();
                for expr in expressions {
                    let res = expr.exec(ctx)?;
                    match res {
                        None => {},
                        Some(mut cmd) => {
                            out += &*String::from_utf8_lossy(&cmd.output().with_context(|| "Failed to read output of command")?.stdout);
                        }
                    }
                }
                ctx.pop_scope();
                Ok(Variable::String(out))
            },
            Value::Values(vec) | Value::ArrayDefinition(vec) => {
                let mut out = Vec::new();
                for val in vec {
                    out.push(val.get(ctx)?);
                }
                Ok(Variable::Array(out))
            }
            Value::ValueFunction(call) => {
                let args = get_variables(ctx, &mut call.args)?;
                let func = ctx.get_func(call.name.as_str()).with_context(|| format!("Function {} not found", call.name))?;
                match func {
                    AnyFunction::Native(func) => {
                        (func.func)(ctx, args)
                    }
                    AnyFunction::UserDefined(_) => todo!("User defined functions are not yet supported")
                }
            }
        }
    }
}

fn get_variables(ctx: &mut vars::Context, args: &mut Vec<Value>) -> Result<Vec<Variable>> {
    let mut out = Vec::new();
    for arg in args {
        out.push(arg.get(ctx)?);
    }
    Ok(out)
}

impl ExecExpression for Expression {
    fn exec(self: &mut Expression, ctx: &mut vars::Context) -> Result<Option<Command>> {
        match self {
            Expression::LetExpression(expr) => expr.exec(ctx),
            Expression::Command(expr) => expr.exec(ctx),
            Expression::JobCommand(_) => todo!("Jobs"),
            Expression::Function(_) => todo!("Function definition"),
            Expression::IfExpression(expr) => expr.exec(ctx),
            Expression::WhileExpression(expr) => expr.exec(ctx),
            Expression::ForExpression(_) => todo!("For expression"),
            Expression::RedirectTargetExpression(expr) => expr.exec(ctx),
            Expression::FileTargetExpression(expr) => expr.exec(ctx),
            Expression::FileSourceExpression(expr) => expr.exec(ctx),
            Expression::Expressions(expr) => expr.exec(ctx),
            Expression::OrExpression(expr) => expr.exec(ctx),
            Expression::AndExpression(expr) => expr.exec(ctx),
            Expression::BreakExpression(expr) => expr.exec(ctx)
        }
    }
}

impl ExecExpression for BreakExpression {
    fn exec(self: &mut BreakExpression, ctx: &mut vars::Context) -> Result<Option<Command>> {
        if ctx.break_num > 0 { ctx.break_num -= 1; return Ok(None) }
        let val = self.num.get(ctx)?.to_string();
        let num: u16 = if !val.is_empty() { val.parse()? } else { 1 };
        ctx.break_num = if num == 0 { 1 } else { num };
        Ok(None)
    }
}

impl ExecExpression for WhileExpression {
    fn exec(self: &mut WhileExpression, ctx: &mut vars::Context) -> Result<Option<Command>> {
        if ctx.break_num > 0 { ctx.break_num -= 1; return Ok(None) }
        let mut condition = match self.condition.exec(ctx)? {
            None => bail!("Invalid while expression"),
            Some(cmd) => cmd
        };
        ctx.add_scope();
        let mut res;
        loop {
            match condition.spawn() {
                Result::Err(_) => {
                    res = Some(condition);
                    break;
                },
                Result::Ok(mut child) => {
                    if !child.wait()?.success() {
                        res = Some(condition);
                        break
                    } else {
                        res = self.contents.exec(ctx)?
                    }
                }
            };
            if ctx.break_num > 0 {
                ctx.break_num -= 1;
                break;
            }
        }
        ctx.pop_scope();

        Ok(res)
    }
}

impl ExecExpression for IfExpression {
    fn exec(self: &mut IfExpression, ctx: &mut vars::Context) -> Result<Option<Command>> {
        if ctx.break_num > 0 { return Ok(None) }
        let mut condition = match self.condition.exec(ctx)? {
            None => bail!("Invalid IF expression"),
            Some(cmd) => cmd
        };
        ctx.add_scope();
        let res = match condition.spawn() {
            Result::Err(_) => {
                self.else_contents.exec(ctx)?
            },
            Result::Ok(mut res) => {
                if !res.wait()?.success() {
                    self.else_contents.exec(ctx)?
                } else {
                    self.contents.exec(ctx)?
                }
            }
        };
        ctx.pop_scope();

        Ok(res)
    }
}

impl ExecExpression for LetExpression {
    fn exec(self: &mut LetExpression, ctx: &mut vars::Context) -> Result<Option<Command>> {
        if ctx.break_num > 0 { return Ok(None) }
        let key = self.key.get(ctx)?;
        let val = self.value.get(ctx)?;
        ctx.set_var(key.to_string(), val);
        Ok(None)
    }
}

impl ExecExpression for Vec<CommandValue> {
    fn exec(self: &mut Vec<CommandValue>, ctx: &mut vars::Context) -> Result<Option<Command>> {
        if ctx.break_num > 0 { return Ok(None) }
        if self.is_empty() { bail!("Command with 0 length"); }
        let mut first = self.remove(0);
        let command_name = first.get(ctx)?.to_string();
        let mut cmd = Command::new(command_name);
        for value in self {
            cmd.arg(value.get(ctx)?.to_string());
        }
        Ok(Some(cmd))
    }
}

impl ExecExpression for RedirectTargetExpression {
    fn exec(self: &mut RedirectTargetExpression, ctx: &mut vars::Context) -> Result<Option<Command>> {
        if ctx.break_num > 0 { return Ok(None) }
        let mut src = self.source.exec(ctx)?.unwrap();
        let mut target = self.target.exec(ctx)?.unwrap();
        src.stdout(Stdio::piped());
        match src.spawn() {
            Result::Err(e) => { println!("Error executing: {}", e)},
            Result::Ok(res) => {
                target.stdin(res.stdout.unwrap());
            }
        }

        Ok(Some(target))
    }
}

impl ExecExpression for FileTargetExpression {
    fn exec(self: &mut FileTargetExpression, ctx: &mut vars::Context) -> Result<Option<Command>> {
        if ctx.break_num > 0 { return Ok(None) }
        let src = &mut self.source;
        let target = self.target.get(ctx)?;
        let src = match src {
            Some(expr) => expr.exec(ctx)?,
            None => {
                todo!("Redirect without target file");
            }
        };
        let command = match src {
            Some(mut cmd) => {
                cmd.stdout(Stdio::piped());
                let file = File::create(target.to_string());
                match file {
                    Result::Err(e) => println!("Error: {}", e),
                    Result::Ok(mut file) => {
                        match cmd.spawn() {
                            Result::Err(e) => {
                                println!("Error executing command: {}", e);
                            },
                            Result::Ok(res) => {
                                io::copy(&mut res.stdout.unwrap(), &mut file)?;
                            }
                        }
                    }
                }
                cmd
            },
            None => { bail!("Invalid command provided for file target"); }
        };
        Ok(Some(command))
    }
}

impl ExecExpression for FileSourceExpression {
    fn exec(self: &mut FileSourceExpression, ctx: &mut Context) -> Result<Option<Command>> {
        if ctx.break_num > 0 { return Ok(None) }
        let source = self.source.get(ctx)?.to_string();
        let target = &mut self.target;
        let target = match target {
            Some(expr) => expr.exec(ctx)?,
            None => {
                Some(Command::new("less"))
            }
        };
        let mut command = match target {
            None => { bail!("Invalid command") },
            Some(cmd) => cmd
        };
        let source = match File::open(source) {
            Result::Err(e) => bail!("Cannot open file: {}", e),
            Result::Ok(file) => file
        };
        command.stdin(source);

        Ok(Some(command))
    }
}

impl ExecExpression for Vec<Expression> {
    fn exec(self: &mut Vec<Expression>, ctx: &mut Context) -> Result<Option<Command>> {
        if ctx.break_num > 0 { return Ok(None) }
        let mut last = None;
        for expr in self {
            last = expr.exec(ctx)?;
            if ctx.break_num > 0 { return Ok(last) }
        }
        Ok(last)
    }
}

impl ExecExpression for OrExpression {
    fn exec(self: &mut OrExpression, ctx: &mut Context) -> Result<Option<Command>> {
        if ctx.break_num > 0 { return Ok(None) }
        let mut first = match self.first.exec(ctx)? {
            None => bail!("Invalid OR expression"),
            Some(cmd) => cmd
        };
        let res = match first.spawn() {
            Result::Err(_) => {
                self.second.exec(ctx)?
            },
            Result::Ok(mut res) => {
                if res.wait()?.success() {
                    Some(first)
                } else {
                    self.second.exec(ctx)?
                }
            }
        };

        Ok(res)
    }
}

impl ExecExpression for AndExpression {
    fn exec(self: &mut AndExpression, ctx: &mut Context) -> Result<Option<Command>> {
        if ctx.break_num > 0 { return Ok(None) }
        let mut first = match self.first.exec(ctx)? {
            None => bail!("Invalid AND expression"),
            Some(cmd) => cmd
        };
        let res = match first.spawn() {
            Result::Err(_) => {
                Some(first)
            },
            Result::Ok(mut res) => {
                if !res.wait()?.success() {
                    Some(first)
                } else {
                    self.second.exec(ctx)?
                }
            }
        };

        Ok(res)
    }
}

pub fn exec_tree(tree: Vec<Expression>, ctx: &mut vars::Context) -> Result<()> {
    for mut expression in tree {
        let cmd = expression.exec(ctx)?;
        match cmd {
            None => {},
            Some(mut cmd) => match cmd.spawn() {
                Result::Err(e) => {
                    println!("Error executing: {}", e);
                },
                Result::Ok(mut res) => {
                    let out = res.wait()?;
                    ctx.set_var(String::from("?"), Variable::I32(out.code().unwrap_or(1)));
                }
            }
        }
        if ctx.break_num > 0 { bail!("Too many break statements") }
    }
    Ok(())
}
