use std::fs::File;
use std::process::{Child, Command, Stdio};
use std::io;
use crate::parser::ast::{AndExpression, BreakExpression, CommandValue, Expression, FileSourceExpression, FileTargetExpression, ForExpression, IfExpression, LetExpression, OrExpression, RedirectTargetExpression, Value, WhileExpression};
use crate::parser::vars;
use crate::parser::vars::{AnyFunction, Context, Variable};
use anyhow::{Result, bail, Context as AnyhowContext};

trait ExecExpression {
    fn exec(&mut self, ctx: &mut vars::Context) -> Result<Option<Command>>;
}

trait GetValue {
    fn get(&mut self, ctx: &mut vars::Context) -> Result<Variable>;
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
            Expression::ForExpression(expr) => expr.exec(ctx),
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

impl ExecExpression for Command {
    fn exec(&mut self, ctx: &mut Context) -> Result<Option<Command>> {
        let overrides = ctx.get_overrides()?;
        if let Some(stdout) = overrides.stdout { self.stdout(stdout); }
        if let Some(stderr) = overrides.stderr { self.stderr(stderr); }
        if let Some(stdin) = overrides.stdin { self.stdin(stdin); }
        let name = self.get_program().to_str().unwrap_or("unknown").to_string();
        let out = self.spawn()
            .with_context(|| "Failed to spawn process ".to_string() + &name)?
            .wait()
            .with_context(|| "Failed to wait for process")?;
        ctx.set_var(String::from("?"), Variable::I32(out.code().unwrap_or(-1)));
        Ok(None)
    }
}

impl ExecExpression for Option<Command> {
    fn exec(&mut self, ctx: &mut Context) -> Result<Option<Command>> {
        match self {
            None => Ok(None),
            Some(cmd) => cmd.exec(ctx)
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
        let mut res = None;
        loop {
            let condres = condition.exec(ctx)?;
            let code = ctx.get_last_exit_code().unwrap_or(1);

            if code == 0 {
                res = self.contents.exec(ctx)?
            } else {
                res = condres;
                break;
            }
            if ctx.break_num > 0 {
                ctx.break_num -= 1;
                break;
            }
        }
        ctx.pop_scope();

        Ok(res)
    }
}

impl ExecExpression for ForExpression {
    fn exec<'a>(&mut self, ctx: &mut Context) -> Result<Option<Command>> {
        if ctx.break_num > 0 { ctx.break_num -= 1; return Ok(None) }
        let arg_value = self.arg_value.get(ctx)?;
        let arg_key = match &self.arg_key {
            None => None,
            Some(key) => {
                let mut lkey: Value = key.clone();
                let res = lkey.get(ctx)?;
                Some(res)
            }
        };
        let mut res: Option<Command> = None;
        let list = self.list.get(ctx)?;

        fn process(i: usize, val: Variable, ctx: &mut Context, arg_key: &Option<Variable>, arg_value: &Variable) -> Result<()> {
            ctx.add_scope();
            if let Some(key) = &arg_key {
                ctx.set_var(key.to_string(), Variable::U64(i as u64));
            }
            ctx.set_var(arg_value.to_string(), val);
            Ok(())
        }

        match list {
            Variable::Array(arr) => {
                if arr.is_empty() {
                    self.else_contents.exec(ctx)?;
                } else {
                    for (i, val) in arr.iter().enumerate() {
                        process(i, val.clone(), ctx, &arg_key, &arg_value)?;
                        res.exec(ctx)?;
                        res = self.contents.exec(ctx)?;
                        ctx.pop_scope();
                        if ctx.break_num > 0 {
                            ctx.break_num -= 1;
                            break;
                        }
                    }
                }
            },
            Variable::String(str) => {
                if str.is_empty() {
                    self.else_contents.exec(ctx)?;
                } else {
                    for (i, char) in str.chars().enumerate() {
                        process(i, Variable::String(char.to_string()), ctx, &arg_key, &arg_value)?;
                        res.exec(ctx)?;
                        res = self.contents.exec(ctx)?;
                        ctx.pop_scope();
                        if ctx.break_num > 0 {
                            ctx.break_num -= 1;
                            break;
                        }
                    }
                }
            },
            _ => bail!("Invalid for expression")
        };

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
        let mut res = condition.exec(ctx)?;
        let code = ctx.get_last_exit_code().unwrap_or(1);
        if code == 0 {
            res = self.contents.exec(ctx)?;
        } else {
            res = self.else_contents.exec(ctx)?;
        }
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
        let first = self.get_mut(0).unwrap();
        let command_name = first.get(ctx)?.to_string();
        let mut cmd = Command::new(command_name);
        for value in &mut self[1..] {
            cmd.arg(value.get(ctx)?.to_string());
        }
        Ok(Some(cmd))
    }
}

impl ExecExpression for RedirectTargetExpression {
    fn exec(self: &mut RedirectTargetExpression, ctx: &mut vars::Context) -> Result<Option<Command>> {
        if ctx.break_num > 0 { return Ok(None) }
        let (reader, writer) = os_pipe::pipe()?;
        let mut src = self.source.exec(ctx)?.unwrap();
        let mut target = self.target.exec(ctx)?.unwrap();
        target.stdin(reader);
        ctx.add_scope();
        ctx.scopes.last_mut().unwrap().stdout_override = Some(writer);
        src.exec(ctx)?;
        ctx.pop_scope();

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
                let file = File::create(target.to_string())?;
                cmd.stdout(file);
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
        let source = File::open(source).with_context(|| "Couldn't open file to read")?;
        command.stdin(source);

        Ok(Some(command))
    }
}

impl ExecExpression for Vec<Expression> {
    fn exec(self: &mut Vec<Expression>, ctx: &mut Context) -> Result<Option<Command>> {
        if ctx.break_num > 0 { return Ok(None) }
        let mut last = None;
        for expr in self {
            last.exec(ctx)?;
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
        first.exec(ctx)?;
        let code = ctx.get_last_exit_code().unwrap_or(1);
        if code == 0 {
            Ok(Some(first))
        } else {
            self.second.exec(ctx)
        }
    }
}

impl ExecExpression for AndExpression {
    fn exec(self: &mut AndExpression, ctx: &mut Context) -> Result<Option<Command>> {
        if ctx.break_num > 0 { return Ok(None) }
        let mut first = match self.first.exec(ctx)? {
            None => bail!("Invalid AND expression"),
            Some(cmd) => cmd
        };
        first.exec(ctx)?;
        let code = ctx.get_last_exit_code().unwrap_or(1);
        if code == 0 {
            self.second.exec(ctx)
        } else {
            Ok(Some(first))
        }
    }
}

pub fn exec_tree(tree: Vec<Expression>, ctx: &mut vars::Context) -> Result<()> {
    for mut expression in tree {
        let mut cmd = expression.exec(ctx)?;
        cmd.exec(ctx)?;
        if ctx.break_num > 0 { bail!("Too many break statements") }
    }
    Ok(())
}
