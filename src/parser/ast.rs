use crate::parser::tokens::Tokens;
use anyhow::{bail, Context, Result};

#[derive(Debug)]
pub struct LetExpression {
    pub key: Box<Value>,
    pub vartype: Option<String>,
    pub value: Box<Value>
}

#[derive(Debug)]
pub struct AndExpression {
    pub first: Box<Expression>,
    pub second: Box<Expression>
}

#[derive(Debug)]
pub struct OrExpression {
    pub first: Box<Expression>,
    pub second: Box<Expression>
}

#[derive(Debug)]
pub struct IfExpression {
    pub condition: Box<Expression>,
    pub contents: Vec<Expression>,
    pub else_contents: Vec<Expression>
}

#[derive(Debug)]
pub struct WhileExpression {
    pub condition: Box<Expression>,
    pub contents: Vec<Expression>
}

#[derive(Debug)]
pub struct ForExpression {
    pub key: String,
    pub list: Box<Value>,
    pub contents: Vec<Expression>
}

#[derive(Debug)]
pub enum ForValue {
    Value(Value),
    Range(Option<u32>, Option<u32>)
}

#[derive(Debug)]
pub struct DefinedFunction {
    name: String,
    args: Vec<Value>,
    body: Vec<Expression>
}

#[derive(Debug)]
pub enum Value {
    Literal(String),
    Variable(String),
    ArrayVariable(String),
    ArrayFunction(DefinedFunction),
    StringFunction(DefinedFunction),
    Expressions(Vec<Expression>),
    Values(Vec<Value>)
}

#[derive(Debug)]
pub struct FunctionVariable {
    pub name: String,
    pub vartype: Option<String>
}

#[derive(Debug)]
pub struct FunctionDefinitionExpression {
    pub name: String,
    pub args: Vec<FunctionVariable>,
    pub body: Box<Expression>
}

#[derive(Debug)]
pub struct RedirectTargetExpression {
    pub source: Box<Expression>,
    pub target: Box<Expression>
}

#[derive(Debug)]
pub struct FileTargetExpression {
    pub source: Option<Box<Expression>>,
    pub target: Box<Value>
}

#[derive(Debug)]
pub struct FileSourceExpression {
    pub source: Box<Value>,
    pub target: Option<Box<Expression>>
}

#[derive(Debug)]
pub enum CommandValue {
    Value(Value),
    Var(String, Value)
}

#[derive(Debug)]
pub struct BreakExpression {
    pub num: Box<Value>
}

#[derive(Debug)]
pub enum Expression {
    LetExpression(LetExpression),
    Command(Vec<CommandValue>),
    JobCommand(Box<Expression>),
    Function(FunctionDefinitionExpression),
    IfExpression(IfExpression),
    WhileExpression(WhileExpression),
    ForExpression(ForExpression),
    RedirectTargetExpression(RedirectTargetExpression),
    FileTargetExpression(FileTargetExpression),
    FileSourceExpression(FileSourceExpression),
    Expressions(Vec<Expression>),
    OrExpression(OrExpression),
    AndExpression(AndExpression),
    BreakExpression(BreakExpression)
}

#[derive(Debug)]
struct Tree {
    tokens: Vec<Tokens>,
    i: usize
}

impl Tree {
    fn parse_call(&mut self, end: usize) -> Result<Expression> {
        let mut values: Vec<CommandValue> = Vec::new();
        let mut buf: Vec<Value> = Vec::new();
        let mut token = self.get_current_token();
        loop {
            if matches!(token, Tokens::Space) {
                if buf.len() > 0 {
                    values.push(CommandValue::Value(Value::Values(buf)));
                    buf = Vec::new();
                }
                if self.i >= end - 1 { break }
                self.i += 1;
                token = self.get_current_token();
                continue;
            }
            let val = match &token {
                Tokens::Literal(str) => Value::Literal(str.clone()),
                Tokens::SubStart => {
                    let val = self.get_value(end)?;
                    token = self.get_current_token();
                    val
                },
                Tokens::StringVariable(str, _) => {
                    if str.len() == 0 { bail!("Expected variable name"); }
                    Value::Variable(str.clone())
                },
                Tokens::ArrayVariable(str, _) => Value::ArrayVariable(str.clone()),
                Tokens::FileWrite => break,
                Tokens::FileRead => break,
                Tokens::RedirectInto => break,
                Tokens::And => break,
                Tokens::Or => break,
                Tokens::JobCommandEnd => break,
                Tokens::ParenthesisEnd => {
                    if self.i >= end - 1 {
                        break;
                    }
                    Value::Literal(token.to_str())
                }
                _ => {
                    Value::Literal(token.to_str())
                }
            };
            buf.push(val);
            if self.i >= end - 1 { break }
            self.i += 1;
            token = self.tokens.get(self.i).unwrap();
            if matches!(token, Tokens::CommandEnd(_)) { break }
        }
        match &token {
            Tokens::FileWrite | Tokens::FileRead | Tokens::RedirectInto => self.i -= 1,
            _ => {}
        }
        // self.next();
        if buf.len() > 0 {
            values.push(CommandValue::Value(Value::Values(buf)));
        }
        Ok(Expression::Command(values))
    }

    fn parse_let(&mut self, end: usize) -> Result<Expression> {
        if end < self.i + 2 { bail!("Let needs name and equal sign (=) at minimum") }
        self.inc();
        let mut len = 0;
        for token in &self.tokens[self.i..] {
            match token {
                Tokens::ExportSet => { break },
                _ => len += 1
            }
        }
        let key = Box::new(self.get_value(self.i + len)?);
        self.inc(); // ????
        self.inc();
        let value = Box::new(self.get_value(end)?);
        Ok(Expression::LetExpression(LetExpression { key, vartype: None, value }))
    }

    fn parse_read(&mut self, target: Option<Expression>, _end: usize) -> Result<Expression> {
        let target = match target {
            Some(source) => Some(Box::new(source)),
            None => None
        };
        self.i += 1;
        let mut val_end = self.i;
        let mut found_first = false;
        for token in &self.tokens[self.i..] {
            val_end += 1;
            match token {
                Tokens::Space => if found_first { break },
                Tokens::CommandEnd(_) => if !found_first { bail!("Unexpected command end") } else { break },
                Tokens::FileRead => bail!("Unexpected file read (<)"),
                Tokens::FileWrite => bail!("Unexpected file write (>)"),
                _ => { found_first = true; }
            }
        }
        val_end -= 1;
        let source = Box::new(self.get_value(val_end)?);
        self.inc();
        Ok(Expression::FileSourceExpression(FileSourceExpression { source, target }))
    }

    fn parse_write(&mut self, source: Option<Expression>, _end: usize) -> Result<Expression> {
        let source = match source {
            Some(source) => Some(Box::new(source)),
            None => None
        };
        self.i += 1;
        let mut val_end = self.i;
        let mut found_first = false;
        for token in &self.tokens[self.i..] {
            val_end += 1;
            match token {
                Tokens::Space => if found_first { break },
                Tokens::CommandEnd(_) => if !found_first { bail!("Unexpected command end") } else { break },
                Tokens::FileRead => bail!("Unexpected file read (<)"),
                Tokens::FileWrite => bail!("Unexpected file write (>)"),
                _ => { found_first = true; }
            }
        }
        val_end -= 1;
        let target = Box::new(self.get_value(val_end)?);
        self.inc();
        Ok(Expression::FileTargetExpression(FileTargetExpression { source, target }))
    }

    fn parse_function(&mut self, _end: usize) -> Result<FunctionDefinitionExpression> {
        bail!("Functions not yet implemented")
    }

    fn parse_array_func(&mut self, _str: String, _end: usize) -> Result<DefinedFunction> {
        bail!("Array functions not yet implemented");
    }

    fn parse_string_func(&mut self, _str: String, _end: usize) -> Result<DefinedFunction> {
        bail!("Array functions not yet implemented");
    }

    fn parse_for(&mut self, _end: usize) -> Result<ForExpression> {
        bail!("For loop not yet implemented");
    }

    fn parse_if(&mut self, end: usize) -> Result<IfExpression> {
        self.inc();
        let condition = self.get_expression(end).with_context(|| "Error getting condition for if expression")?;
        dbg!(&condition);
        let mut contents = Vec::new();
        loop {
            match self.get_next_token() {
                Tokens::End => break,
                Tokens::Space => { self.inc(); },
                _ => contents.push(self.get_expression(end).with_context(|| "Error getting contents for if expression")?)
            };
        }
        let mut else_contents = Vec::new();
        Ok(IfExpression { condition: Box::new(condition), contents, else_contents })
    }

    fn parse_while(&mut self, end: usize) -> Result<WhileExpression> {
        self.inc();
        let condition = self.get_expression(end).with_context(|| "Error getting condition for while expression")?;
        dbg!(&condition);
        dbg!(self.i);
        let mut contents = Vec::new();
        loop {
            let token = self.get_next_token();
            dbg!(token);
            match token {
                Tokens::End => break,
                Tokens::Space => { self.inc(); },
                _ => contents.push(self.get_expression(end).with_context(|| "Error getting contents for while expression")?)
            };
            dbg!(&contents);
        }
        Ok(WhileExpression { condition: Box::new(condition), contents })
    }

    fn parse_sub(&mut self, end: usize) -> Result<Vec<Expression>> {
        let mut expressions: Vec<Expression> = Vec::new();
        loop {
            if self.i >= end - 1 { break; }
            expressions.push(self.get_expression(end)?);
        }
        Ok(expressions)
    }

    fn get_value(&mut self, end: usize) -> Result<Value> {
        let mut token = self.get_current_token();
        let mut values: Vec<Value> = Vec::new();
        let mut buf: Vec<Value> = Vec::new();
        loop {
            match token {
                Tokens::Space => {
                    if buf.len() == 0 { token = self.inc().get_current_token(); continue; }
                    values.push(Value::Values(buf));
                    buf = Vec::new();
                    if self.i >= end - 1 { break }
                },
                Tokens::CommandEnd(_) => break,
                Tokens::Literal(str) => buf.push(Value::Literal(str.clone())),
                Tokens::ExportSet => bail!("Unexpected token EXPORT_SET (=)"),
                Tokens::FileRead => buf.push(Value::Literal(token.to_str())),
                Tokens::Function => buf.push(Value::Literal(token.to_str())),
                Tokens::FileWrite => buf.push(Value::Literal(token.to_str())),
                Tokens::RedirectInto => bail!("Unexpected token REDIRECT (|)"),
                Tokens::ParenthesisEnd => bail!("Unexpected token FUNCTION CALL END ())"),
                Tokens::ArrayFunction(_) => bail!("Unexpected array function"),
                Tokens::StringFunction(_) => bail!("Unexpected string function"),
                Tokens::ParenthesisStart => bail!("Parenthesis not yet implemented"),
                Tokens::SubStart => {
                    let mut len = 0;
                    let mut lvl = 1;
                    self.inc();
                    for token in &self.tokens[self.i..] {
                        match token {
                            Tokens::SubStart => lvl += 1,
                            Tokens::StringFunction(_) => lvl += 1,
                            Tokens::ArrayFunction(_) => lvl += 1,
                            Tokens::ParenthesisStart => lvl += 1,
                            Tokens::ParenthesisEnd => lvl -= 1,
                            _ => {}
                        }
                        if lvl == 0 {
                            break;
                        }

                        if len + self.i == end { break }
                        len += 1;
                    }
                    // self.inc();
                    if lvl != 0 {
                        bail!("Parenthesis do not match");
                    }
                    dbg!(&self, len);
                    let val = Value::Expressions(self.parse_sub(self.i + len)?);
                    self.inc();
                    dbg!(self);
                    return Ok(val);
                },
                Tokens::Else => buf.push(Value::Literal(token.to_str())),
                Tokens::End => buf.push(Value::Literal(token.to_str())),
                Tokens::For => buf.push(Value::Literal(token.to_str())),
                Tokens::If => buf.push(Value::Literal(token.to_str())),
                Tokens::Let => buf.push(Value::Literal(token.to_str())),
                Tokens::While => buf.push(Value::Literal(token.to_str())),
                Tokens::StringVariable(str, _) => {
                    if buf.len() != 0 {
                        values.push(Value::Values(buf));
                        buf = Vec::new();
                    }
                    values.push(Value::Variable(str.clone()));
                },
                Tokens::ArrayVariable(str, _) => {
                    if buf.len() != 0 {
                        values.push(Value::Values(buf));
                        buf = Vec::new();
                    }
                    values.push(Value::ArrayVariable(str.clone()));
                },
                Tokens::And => bail!("Unexpected AND (&&)"),
                Tokens::Or => bail!("Unexpected OR (||)"),
                Tokens::Break => buf.push(Value::Literal(token.to_str())),
                Tokens::JobCommandEnd => bail!("Unexpected job command end (&)"),
            }
            if self.i >= end - 1 { break }
            token = self.inc().get_current_token();
        }
        if buf.len() > 0 {
            values.push(Value::Values(buf));
        }
        Ok(Value::Values(values))
    }

    fn get_expression(&mut self, end: usize) -> Result<Expression> {
        let mut token = self.get_current_token();
        let mut expr: Option<Expression> = None;
        loop {
            match token {
                Tokens::Space => {self.inc();},
                Tokens::CommandEnd(_) => { if matches!(expr, Some(_)) { break }; self.inc();},
                Tokens::Literal(_) => if matches!(expr, Some(_)) {
                    bail!("Unexpected literal. After file redirect, you need to use a semicolon or newline.");
                } else {
                    expr = Some(self.parse_call(end)?);
                },
                Tokens::ExportSet => bail!("Unexpected token EXPORT SET (=)"),
                Tokens::Function => return Ok(Expression::Function(self.parse_function(end)?)),
                Tokens::FileRead => expr = Some(self.parse_read(expr, end)?),
                Tokens::FileWrite => expr = Some(self.parse_write(expr, end)?),
                Tokens::RedirectInto => match expr {
                    None => bail!("Unexpected token REDIRECT (|)"),
                    Some(_) => {
                        self.i += 1;
                        expr = Some(Expression::RedirectTargetExpression(RedirectTargetExpression { source: Box::new(expr.unwrap()), target: Box::new(self.get_expression(end)?) }));
                    }
                },
                Tokens::ParenthesisStart => if matches!(expr, Some(_)) {
                    bail!("Unexpected parenthesis. After file redirect, you need to use a semicolon or newline.");
                } else {
                    let mut len = 1;
                    let mut lvl = 1;
                    self.inc();
                    for token in &self.tokens[self.i..] {
                        match token {
                            Tokens::ParenthesisStart => lvl += 1,
                            Tokens::ParenthesisEnd => lvl -= 1,
                            _ => {}
                        }
                        if lvl == 0 {
                            break;
                        }

                        if len + self.i == end { break }
                        len += 1;
                    }
                    if lvl != 0 {
                        bail!("Parenthesis not ended properly.");
                    }
                    expr = Some(self.get_expression(self.i + len)?);
                    self.inc();
                },
                Tokens::ParenthesisEnd => bail!("Unexpected token PARENTHESIS END ())"),
                Tokens::ArrayFunction(_) => bail!("Unexpected array function"),
                Tokens::StringFunction(_) => bail!("Unexpected string function"),
                Tokens::SubStart => match expr {
                    Some(_) => bail!("Unexpected literal. After file redirect, you need to use a semicolon or newline."),
                    _ => expr = Some(self.parse_call(end)?)
                },
                Tokens::Else => bail!("Unexpected token ELSE"),
                Tokens::End => bail!("Unexpected token END\nCurrent expression:{:?}", expr),
                Tokens::For => match expr {
                    Some(_) => bail!("Commands must be ended properly"),
                    None => expr = Some(Expression::ForExpression(self.parse_for(end)?)),
                },
                Tokens::If => match expr {
                    Some(_) => bail!("Commands must be ended properly"),
                    None => expr = Some(Expression::IfExpression(self.parse_if(end)?)),
                }
                Tokens::Let => return Ok(self.parse_let(end)?),
                Tokens::While => return Ok(Expression::WhileExpression(self.parse_while(end)?)),
                Tokens::StringVariable(_, _) => if matches!(expr, Some(_)) {
                    bail!("Unexpected variable. After file redirect, you need to use a semicolon or newline.");
                } else {
                    expr = Some(self.parse_call(end)?);
                },
                Tokens::ArrayVariable(_, _) => bail!("Unexpected array variable"),
                Tokens::And => match expr {
                    None => bail!("Unexpected AND (&&)"),
                    Some(_) => {
                        self.inc();
                        expr = Some(Expression::AndExpression(AndExpression { first: Box::new(expr.unwrap()), second: Box::new(self.get_expression(end)?) }));
                    }
                },
                Tokens::Or => match expr {
                    None => bail!("Unexpected OR (||)"),
                    Some(_) => {
                        self.inc();
                        expr = Some(Expression::OrExpression(OrExpression { first: Box::new(expr.unwrap()), second: Box::new(self.get_expression(end)?) }));
                    }
                },
                Tokens::Break => match expr {
                    None => {
                        expr = Some(Expression::BreakExpression(BreakExpression { num: Box::new(self.get_value(end)?)}));
                    },
                    Some(_) => bail!("Unexpected break")
                }
                Tokens::JobCommandEnd => bail!("Jobs not yet implemented")
            }
            if self.i >= end - 1 { break }
            token = self.get_current_token();
        }
        match expr {
            Some(expr) => Ok(expr),
            None => bail!("No expression found")
        }
    }

    fn inc(&mut self) -> &Self {
        self.i += 1;
        self
    }
    fn get_current_token(&self) -> &Tokens { self.tokens.get(self.i).unwrap() }
    fn get_next_token(&self) -> &Tokens { self.tokens.get(self.i + 1).unwrap() }
}

pub fn build_tree(tokens: Vec<Tokens>) -> Result<Vec<Expression>> {
    let mut expressions: Vec<Expression> = Vec::new();
    let mut tree = Tree { tokens, i: 0 };
    loop {
        if tree.i >= tree.tokens.len() - 1 { break; }
        let val = tree.get_expression(tree.tokens.len());
        match val {
            Ok(val) => expressions.push(val),
            Err(error) => {
                dbg!(tree);
                return Err(error);
            }
        }
    }
    Ok(expressions)
}
