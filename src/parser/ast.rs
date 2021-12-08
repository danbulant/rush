use crate::parser::tokens::Tokens;

#[derive(Debug)]
pub struct LetExpression {
    pub key: Box<Value>,
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
    pub contents: Vec<Expression>
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
    Expression(Expression),
    Values(Vec<Value>)
}

#[derive(Debug)]
pub struct FunctionDefinitionExpression {
    pub args: Vec<String>,
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
    AndExpression(AndExpression)
}

#[derive(Debug)]
struct Tree {
    tokens: Vec<Tokens>,
    i: usize
}

impl Tree {
    fn parse_call(&mut self, end: usize) -> Expression {
        let mut values: Vec<CommandValue> = Vec::new();
        let mut buf: Vec<Value> = Vec::new();
        dbg!("call parse");
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
                dbg!("skip space");
                continue;
            }
            let val = match &token {
                Tokens::Literal(str) => Value::Literal(str.clone()),
                Tokens::SubStart => {
                    let val = self.get_value(end);
                    token = self.get_current_token();
                    val
                },
                Tokens::StringVariable(str, _) => Value::Variable(str.clone()),
                Tokens::ArrayVariable(str, _) => Value::ArrayVariable(str.clone()),
                Tokens::FileWrite => break,
                Tokens::FileRead => break,
                Tokens::RedirectInto => break,
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
            dbg!("parse call loop");
        }
        dbg!(&token);
        match &token {
            Tokens::FileWrite | Tokens::FileRead | Tokens::RedirectInto => self.i -= 1,
            _ => {}
        }
        dbg!(&self);
        // self.next();
        if buf.len() > 0 {
            values.push(CommandValue::Value(Value::Values(buf)));
        }
        Expression::Command(values)
    }

    fn parse_let(&mut self, end: usize) -> Expression {
        if end < self.i + 2 { panic!("Let needs name and equal sign (=) at minimum") }
        self.inc();
        let mut len = 0;
        for token in &self.tokens[self.i..] {
            match token {
                Tokens::ExportSet => { break },
                _ => len += 1
            }
        }
        let key = Box::new(self.get_value(self.i + len));
        self.inc(); // ????
        self.inc();
        let value = Box::new(self.get_value(end));
        Expression::LetExpression(LetExpression { key, value })
    }

    fn parse_read(&mut self, target: Option<Expression>, end: usize) -> Expression {
        let target = match target {
            Some(source) => Some(Box::new(source)),
            None => None
        };
        self.i += 1;
        let mut val_end = self.i;
        let mut found_first = false;
        for token in &self.tokens[self.i..] {
            dbg!(&token);
            val_end += 1;
            match token {
                Tokens::Space => if found_first { break },
                Tokens::CommandEnd(_) => if !found_first { panic!("Unexpected command end") } else { break },
                Tokens::FileRead => panic!("Unexpected file read (<)"),
                Tokens::FileWrite => panic!("Unexpected file write (>)"),
                _ => { found_first = true; }
            }
        }
        dbg!(&self.tokens[self.i..val_end]);
        val_end -= 1;
        let source = Box::new(self.get_value(val_end));
        self.inc();
        Expression::FileSourceExpression(FileSourceExpression { source, target })
    }

    fn parse_write(&mut self, source: Option<Expression>, end: usize) -> Expression {
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
                Tokens::CommandEnd(_) => if !found_first { panic!("Unexpected command end") } else { break },
                Tokens::FileRead => panic!("Unexpected file read (<)"),
                Tokens::FileWrite => panic!("Unexpected file write (>)"),
                _ => { found_first = true; }
            }
        }
        val_end -= 1;
        let target = Box::new(self.get_value(val_end));
        self.inc();
        Expression::FileTargetExpression(FileTargetExpression { source, target })
    }

    fn parse_function(&self, end: usize) -> FunctionDefinitionExpression {
        panic!("Functions not yet implemented")
    }

    fn parse_array_func(&self, str: String, end: usize) -> DefinedFunction {
        panic!("Array functions not yet implemented");
    }

    fn parse_string_func(&self, str: String, end: usize) -> DefinedFunction {
        panic!("Array functions not yet implemented");
    }

    fn parse_for(&self, end: usize) -> ForExpression {
        panic!("For loop not yet implemented");
    }

    fn parse_if(&self, end: usize) -> IfExpression {
        panic!("If not yet implemented");
    }

    fn parse_while(&self, end: usize) -> WhileExpression {
        panic!("While not yet implemented");
    }

    fn parse_sub(&mut self, end: usize) -> Vec<Expression> {
        let mut expressions: Vec<Expression> = Vec::new();
        loop {
            if self.i >= end - 1 { break; }
            expressions.push(self.get_expression(end));
            dbg!(&expressions);
        }
        expressions
    }

    fn get_value(&mut self, end: usize) -> Value {
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
                Tokens::ExportSet => panic!("Unexpected token EXPORT_SET (=)"),
                Tokens::FileRead => buf.push(Value::Literal(token.to_str())),
                Tokens::Function => buf.push(Value::Literal(token.to_str())),
                Tokens::FileWrite => buf.push(Value::Literal(token.to_str())),
                Tokens::RedirectInto => panic!("Unexpected token REDIRECT (|)"),
                Tokens::ParenthesisEnd => panic!("Unexpected token FUNCTION CALL END ())"),
                Tokens::ArrayFunction(_) => panic!("Unexpected array function"),
                Tokens::StringFunction(_) => panic!("Unexpected string function"),
                Tokens::ParenthesisStart => panic!("Parenthesis not yet implemented"),
                Tokens::SubStart => {
                    let mut len = 1;
                    let mut lvl = 1;
                    self.inc();
                    for token in &self.tokens[self.i..] {
                        match token {
                            Tokens::SubStart => lvl += 1,
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
                    dbg!(&self);
                    dbg!(len, lvl);
                    if lvl != 0 {
                        panic!("Sub not ended properly");
                    }
                    return Value::Expressions(self.parse_sub(self.i + len));
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
                Tokens::And => panic!("Unexpected AND (&&)"),
                Tokens::Or => panic!("Unexpected OR (||)"),
                Tokens::JobCommandEnd => panic!("Unexpected job command end (&)"),
            }
            if self.i >= end - 1 { break }
            token = self.inc().get_current_token();
        }
        if buf.len() > 0 {
            values.push(Value::Values(buf));
        }
        Value::Values(values)
    }

    fn get_expression(&mut self, end: usize) -> Expression {
        let mut token = self.get_current_token();
        dbg!("expr");
        let mut expr: Option<Expression> = None;
        loop {
            dbg!(&token);
            match token {
                Tokens::Space => {self.inc();},
                Tokens::CommandEnd(_) => {self.inc();},
                Tokens::Literal(t) => if matches!(expr, Some(_)) {
                    dbg!(t);
                    dbg!(expr);
                    panic!("Unexpected literal. After file redirect, you need to use a semicolon or newline.");
                } else {
                    expr = Some(self.parse_call(end));
                    dbg!(&self);
                },
                Tokens::ExportSet => panic!("Unexpected token EXPORT_SET (=)"),
                Tokens::Function => return Expression::Function(self.parse_function(end)),
                Tokens::FileRead => expr = Some(self.parse_read(expr, end)),
                Tokens::FileWrite => expr = Some(self.parse_write(expr, end)),
                Tokens::RedirectInto => match expr {
                    None => panic!("Unexpected token REDIRECT (|)"),
                    Some(_) => {
                        self.i += 1;
                        dbg!(&self);
                        expr = Some(Expression::RedirectTargetExpression(RedirectTargetExpression { source: Box::new(expr.unwrap()), target: Box::new(self.get_expression(end)) }));
                        dbg!("after redirect");
                    }
                },
                Tokens::ParenthesisStart => if matches!(expr, Some(_)) {
                    dbg!(expr);
                    panic!("Unexpected parenthesis. After file redirect, you need to use a semicolon or newline.");
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
                        panic!("Parenthesis not ended properly.");
                    }
                    expr = Some(self.get_expression(self.i + len));
                    dbg!(&self);
                },
                Tokens::ParenthesisEnd => panic!("Unexpected token PARENTHESIS END ())"),
                Tokens::ArrayFunction(_) => panic!("Unexpected array function"),
                Tokens::StringFunction(_) => panic!("Unexpected string function"),
                Tokens::SubStart => return self.parse_call(end),
                Tokens::Else => panic!("Unexpected token ELSE"),
                Tokens::End => panic!("Unexpected token END"),
                Tokens::For => return Expression::ForExpression(self.parse_for(end)),
                Tokens::If => return Expression::IfExpression(self.parse_if(end)),
                Tokens::Let => return self.parse_let(end),
                Tokens::While => return Expression::WhileExpression(self.parse_while(end)),
                Tokens::StringVariable(_, _) => if matches!(expr, Some(_)) {
                    dbg!(expr);
                    panic!("Unexpected variable. After file redirect, you need to use a semicolon or newline.");
                } else {
                    expr = Some(self.parse_call(end));
                    dbg!(&self);
                },
                Tokens::ArrayVariable(_, _) => panic!("Unexpected array variable"),
                Tokens::And => panic!("And not yet implemented"),
                Tokens::Or => panic!("Or not yet implemented"),
                Tokens::JobCommandEnd => panic!("Jobs not yet implemented")
            }
            if self.i >= end - 1 { break }
            token = self.get_current_token();
        }
        expr.unwrap()
    }

    fn inc(&mut self) -> &Self {
        self.i += 1;
        self
    }
    fn get_current_token(&self) -> &Tokens {
        self.tokens.get(self.i).unwrap()
    }
    fn get_next_token(&self) -> &Tokens {
        self.tokens.get(self.i + 1).unwrap()
    }
}

pub fn build_tree(tokens: Vec<Tokens>) -> Vec<Expression> {
    println!("Building tree");
    let mut expressions: Vec<Expression> = Vec::new();
    let mut tree = Tree { tokens, i: 0 };
    loop {
        if tree.i == tree.tokens.len() - 1 { break; }
        let val = tree.get_expression(tree.tokens.len());
        expressions.push(val);
    }
    dbg!(&expressions);

    expressions
}
