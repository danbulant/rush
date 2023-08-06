use std::fs::File;
use std::io::Read;
use std::process::Command;
use std::thread;
use crate::parser::ast::{AndExpression, BreakExpression, CommandValue, Expression, FileSourceExpression, FileTargetExpression, ForExpression, IfExpression, LetExpression, OrExpression, RedirectTargetExpression, Value, WhileExpression};
use crate::parser::vars::{AnyFunction, Context, ReaderOverride, Variable, WriterOverride};
use anyhow::{Result, bail, Context as AnyhowContext};

#[derive(Debug, Default)]
struct ExecResult {
    commands: Vec<Command>
}

impl ExecResult {
    fn exec(self, ctx: &mut Context) -> Result<Option<i32>> {
        let mut children = Vec::new();
        for mut command in self.commands {
            let name = command.get_program().to_str().unwrap_or("unknown").to_string();
            let out = command.spawn()
                .with_context(|| "Failed to spawn process ".to_string() + &name)?;
            drop(command);
            children.push(out);
        }
        let mut code = None;
        for mut child in children {
            let out = child.wait()
                .with_context(|| "Command failed")?;
            code = Some(out.code().unwrap_or(-1));
        }
        if let Some(code) = code {
            ctx.set_var(String::from("?"), Variable::I32(code));
        }
        Ok(code)
    }

    fn merge(&mut self, mut other: ExecResult) {
        self.commands.append(&mut other.commands);
    }
}

trait ExecExpression {
    fn exec(&mut self, ctx: &mut Context) -> Result<ExecResult>;
}

trait GetValue {
    fn get(&mut self, ctx: &mut Context) -> Result<Variable>;
}

impl GetValue for CommandValue {
    fn get(self: &mut CommandValue, ctx: &mut Context) -> Result<Variable> {
        match self {
            CommandValue::Value(val) => val.get(ctx),
            CommandValue::Var(_, _) => bail!("Broken executor")
        }
    }
}

impl GetValue for Value {
    fn get(self: &mut Value, ctx: &mut Context) -> Result<Variable> {
        match self {
            Value::Literal(str) => {
                Ok(Variable::String(str.clone()))
            },
            Value::Variable(str) => Ok(ctx.get_var(str).unwrap_or(&mut Variable::String(String::from(""))).clone()),
            Value::ArrayVariable(str) => Ok(ctx.get_var(str).unwrap_or(&mut Variable::Array(Vec::new())).clone()),
            Value::Expressions(expressions) => {
                ctx.add_scope();
                let (mut reader, writer) = os_pipe::pipe()?;
                let mut data = String::new();
                thread::scope(|s| -> Result<()> {
                    ctx.scopes.last_mut().unwrap().stdout_override = Some(WriterOverride::Pipe(writer));
                    s.spawn(|| -> Result<()> {
                        let mut buf = Vec::new();
                        reader.read_to_end(&mut buf)?;
                        data = String::from_utf8_lossy(&buf).to_string();
                        Ok(())
                    });
                    expressions.exec(ctx)?.exec(ctx)?;
                    Ok(())
                })?;
                ctx.pop_scope();
                Ok(Variable::String(data))
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

fn get_variables(ctx: &mut Context, args: &mut Vec<Value>) -> Result<Vec<Variable>> {
    let mut out = Vec::new();
    for arg in args {
        out.push(arg.get(ctx)?);
    }
    Ok(out)
}

impl ExecExpression for Expression {
    fn exec(self: &mut Expression, ctx: &mut Context) -> Result<ExecResult> {
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

impl ExecExpression for BreakExpression {
    fn exec(self: &mut BreakExpression, ctx: &mut Context) -> Result<ExecResult> {
        if ctx.break_num > 0 { ctx.break_num -= 1; return Ok(ExecResult::default()) }
        let val = self.num.get(ctx)?.to_string();
        let num: u16 = if !val.is_empty() { val.parse()? } else { 1 };
        ctx.break_num = if num == 0 { 1 } else { num };
        Ok(ExecResult::default())
    }
}

impl ExecExpression for WhileExpression {
    fn exec(self: &mut WhileExpression, ctx: &mut Context) -> Result<ExecResult> {
        if ctx.break_num > 0 { ctx.break_num -= 1; return Ok(ExecResult::default()) }
        ctx.add_scope();
        let mut res: Option<ExecResult> = None;
        loop {
            let condition = self.condition.exec(ctx)?;
            let condition_res = condition.exec(ctx)?;
            let code = condition_res.unwrap_or(1);

            if code == 0 {
                if let Some(result) = res {
                    result.exec(ctx)?;
                }
                res = Some(self.contents.exec(ctx)?);
            } else {
                res = None;
                break;
            }
            if ctx.break_num > 0 {
                ctx.break_num -= 1;
                break;
            }
        }
        ctx.pop_scope();

        Ok(res.unwrap_or(ExecResult::default()))
    }
}

impl ExecExpression for ForExpression {
    fn exec<'a>(&mut self, ctx: &mut Context) -> Result<ExecResult> {
        if ctx.break_num > 0 { ctx.break_num -= 1; return Ok(ExecResult::default()) }
        let arg_value = self.arg_value.get(ctx)?;
        let arg_key = match &self.arg_key {
            None => None,
            Some(key) => {
                let mut lkey: Value = key.clone();
                let res = lkey.get(ctx)?;
                Some(res)
            }
        };
        let mut res: Option<ExecResult> = None;
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
                        if let Some(res) = res {
                            res.exec(ctx)?;
                        }
                        res = Some(self.contents.exec(ctx)?);
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
                        if let Some(res) = res {
                            res.exec(ctx)?;
                        }
                        res = Some(self.contents.exec(ctx)?);
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

        Ok(res.unwrap_or(ExecResult::default()))
    }
}

impl ExecExpression for IfExpression {
    fn exec(self: &mut IfExpression, ctx: &mut Context) -> Result<ExecResult> {
        if ctx.break_num > 0 { return Ok(ExecResult::default()) }
        let condition = self.condition.exec(ctx)?;
        ctx.add_scope();
        let condition_result = condition.exec(ctx)?;
        let code = condition_result.unwrap_or(1);
        let res= if code == 0 {
            self.contents.exec(ctx)?
        } else {
            self.else_contents.exec(ctx)?
        };
        ctx.pop_scope();

        Ok(res)
    }
}

impl ExecExpression for LetExpression {
    fn exec(self: &mut LetExpression, ctx: &mut Context) -> Result<ExecResult> {
        if ctx.break_num > 0 { return Ok(ExecResult::default()) }
        let key = self.key.get(ctx)?;
        let val = self.value.get(ctx)?;
        ctx.set_var(key.to_string(), val);
        Ok(ExecResult::default())
    }
}

impl ExecExpression for Vec<CommandValue> {
    fn exec(self: &mut Vec<CommandValue>, ctx: &mut Context) -> Result<ExecResult> {
        if ctx.break_num > 0 { return Ok(ExecResult::default()) }
        if self.is_empty() { bail!("Command with 0 length"); }
        let first = self.get_mut(0).unwrap();
        let command_name = first.get(ctx)?.to_string();
        let mut cmd = Command::new(command_name);
        for value in &mut self[1..] {
            cmd.arg(value.get(ctx)?.to_string());
        }
        let overrides = ctx.get_overrides()?;
        if let Some(stdout) = overrides.stdout { cmd.stdout(stdout); }
        if let Some(stderr) = overrides.stderr { cmd.stderr(stderr); }
        if let Some(stdin) = overrides.stdin { cmd.stdin(stdin); }
        Ok(ExecResult {
            commands: vec![cmd]
        })
    }
}

impl ExecExpression for RedirectTargetExpression {
    fn exec(self: &mut RedirectTargetExpression, ctx: &mut Context) -> Result<ExecResult> {
        if ctx.break_num > 0 { return Ok(ExecResult::default()) }
        let (reader, writer) = os_pipe::pipe()?;

        ctx.add_scope();
        ctx.scopes.last_mut().unwrap().stdout_override = Some(WriterOverride::Pipe(writer));
        let mut src = self.source.exec(ctx)?;
        ctx.pop_scope();
        ctx.add_scope();
        ctx.scopes.last_mut().unwrap().stdin_override = Some(ReaderOverride::Pipe(reader));
        let target = self.target.exec(ctx)?;
        ctx.pop_scope();
        src.merge(target);

        Ok(src)
    }
}

impl ExecExpression for FileTargetExpression {
    fn exec(self: &mut FileTargetExpression, ctx: &mut Context) -> Result<ExecResult> {
        if ctx.break_num > 0 { return Ok(ExecResult::default()) }
        let src = &mut self.source;
        let target = self.target.get(ctx)?;

        ctx.add_scope();

        let file = File::create(target.to_string())?;
        ctx.scopes.last_mut().unwrap().stdout_override = Some(WriterOverride::File(file));

        let src = match src {
            Some(expr) => expr.exec(ctx)?,
            None => {
                bail!("Redirect without target file");
            }
        };

        ctx.pop_scope();
        Ok(src)
    }
}

impl ExecExpression for FileSourceExpression {
    fn exec(self: &mut FileSourceExpression, ctx: &mut Context) -> Result<ExecResult> {
        if ctx.break_num > 0 { return Ok(ExecResult::default()) }
        let source = self.source.get(ctx)?.to_string();
        let source = File::open(source).with_context(|| "Couldn't open file to read")?;
        let target = &mut self.target;

        ctx.add_scope();
        ctx.scopes.last_mut().unwrap().stdin_override = Some(ReaderOverride::File(source));
        let target = match target {
            Some(expr) => expr.exec(ctx)?,
            None => {
                vec![CommandValue::Value(Value::Literal(String::from("less")))].exec(ctx)?
            }
        };
        ctx.pop_scope();

        Ok(target)
    }
}

impl ExecExpression for Vec<Expression> {
    fn exec(self: &mut Vec<Expression>, ctx: &mut Context) -> Result<ExecResult> {
        if ctx.break_num > 0 { return Ok(ExecResult::default()) }
        let mut last: Option<ExecResult> = None;
        for expr in self {
            if let Some(last) = last {
                last.exec(ctx)?;
            }
            last = Some(expr.exec(ctx)?);
            if ctx.break_num > 0 { return Ok(last.unwrap()) }
        }
        Ok(last.unwrap_or(ExecResult::default()))
    }
}

impl ExecExpression for OrExpression {
    fn exec(self: &mut OrExpression, ctx: &mut Context) -> Result<ExecResult> {
        if ctx.break_num > 0 { return Ok(ExecResult::default()) }
        let first = self.first.exec(ctx)?;
        let code = first.exec(ctx)?;
        let code = code.unwrap_or(1);
        if code == 0 {
            Ok(ExecResult::default())
        } else {
            self.second.exec(ctx)
        }
    }
}

impl ExecExpression for AndExpression {
    fn exec(self: &mut AndExpression, ctx: &mut Context) -> Result<ExecResult> {
        if ctx.break_num > 0 { return Ok(ExecResult::default()) }
        let first = self.first.exec(ctx)?;
        let code = first.exec(ctx)?;
        let code = code.unwrap_or(1);
        if code == 0 {
            self.second.exec(ctx)
        } else {
            Ok(ExecResult::default())
        }
    }
}

pub fn exec_tree(tree: Vec<Expression>, ctx: &mut Context) -> Result<()> {
    for mut expression in tree {
        let cmd = expression.exec(ctx)?;
        cmd.exec(ctx)?;
        if ctx.break_num > 0 { bail!("Too many break statements") }
    }
    Ok(())
}
