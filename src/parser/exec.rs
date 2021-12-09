use std::error::Error;
use std::process::{Command, Stdio};
use std::io;
use std::ops::Deref;
use crate::parser::ast::{CommandValue, Expression, LetExpression, RedirectTargetExpression, Value};
use crate::parser::vars;
use crate::parser::vars::Variable;

trait ExecExpression {
    fn exec(self, ctx: &mut vars::Context) -> Option<Command>;
}

trait GetValue {
    fn get(self, ctx: &mut vars::Context) -> Variable;
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
            Expression::JobCommand(_) => panic!("Not implemented yet"),
            Expression::Function(_) => panic!("Not implemented yet"),
            Expression::IfExpression(_) => panic!("Not implemented yet"),
            Expression::WhileExpression(_) => panic!("Not implemented yet"),
            Expression::ForExpression(_) => panic!("Not implemented yet"),
            Expression::RedirectTargetExpression(expr) => expr.exec(ctx),
            Expression::FileTargetExpression(_) => panic!("Not implemented yet"),
            Expression::FileSourceExpression(_) => panic!("Not implemented yet"),
            Expression::Expressions(_) => panic!("Not implemented yet"),
            Expression::OrExpression(_) => panic!("Not implemented yet"),
            Expression::AndExpression(_) => panic!("Not implemented yet")
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

pub fn exec_tree(tree: Vec<Expression>, ctx: &mut vars::Context) {
    println!("Executing");
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
                    ctx.set_var(String::from("!"), Variable::I32(out.code().unwrap_or(1)));
                }
            }
        }
    }
}
