use crate::parser::tokens::Tokens;
use crate::parser::vars::Variable;

#[derive(Debug)]
pub struct LetExpression {
    pub key: String,
    pub value: Box<Value>
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
    Function(FunctionDefinitionExpression),
    IfExpression(IfExpression),
    WhileExpression(WhileExpression),
    ForExpression(ForExpression),
    RedirectTargetExpression(RedirectTargetExpression),
    FileTargetExpression(FileTargetExpression),
    FileSourceExpression(FileSourceExpression)
}

struct Tree {
    tokens: Vec<Tokens>,
    i: usize
}

impl Tree {
    fn parse_call(&mut self) -> Vec<CommandValue> {
        let mut values: Vec<CommandValue> = Vec::new();
        let mut token = self.get_current_token();
        let mut buf: Vec<Value> = Vec::new();

        loop {
            if matches!(token, Tokens::Space) {
                if buf.len() > 0 {
                    values.push(CommandValue::Value(Value::Values(buf)));
                    buf = Vec::new();
                }
                token = self.next();
                continue;
            }
            let val = match token {
                Tokens::Literal(str) => Value::Literal(str.clone()),
                Tokens::SubStart => Value::Expressions(self.parse_sub()),
                Tokens::StringVariable(str, _) => Value::Variable(str.clone()),
                Tokens::ArrayVariable(str, _) => Value::ArrayVariable(str.clone()),
                Tokens::FileWrite => {

                }
                _ => {
                    Value::Literal(token.to_str())
                }
            };
            buf.push(val);
            if self.i == self.tokens.len() { break }
            token = self.next();
            if matches!(token, Tokens::CommandEnd(_)) { break }
        }
        if buf.len() > 0 {
            values.push(CommandValue::Value(Value::Values(buf)));
        }
        values
    }

    fn parse_let(&self) -> LetExpression {
        panic!("Let not yet implemented");
    }

    fn parse_read(&self) -> FileSourceExpression {
        panic!("Read not yet implemented");
    }

    fn parse_write(&self, source: Option<Expression>) -> FileTargetExpression {
        let source = match source {
            Some(source) => Some(Box::new(source)),
            None => None
        };

    }

    fn parse_function(&self) -> FunctionDefinitionExpression {
        panic!("Functions not yet implemented")
    }

    fn parse_array_func(&self, str: String) -> DefinedFunction {
        panic!("Array functions not yet implemented");
    }

    fn parse_for(&self) -> ForExpression {
        panic!("For loop not yet implemented");
    }

    fn parse_if(&self) -> IfExpression {
        panic!("If not yet implemented");
    }

    fn parse_while(&self) -> WhileExpression {
        panic!("While not yet implemented");
    }

    fn parse_sub(&self) -> Vec<Expression> {
        panic!("Sub not yet implemented")
    }

    fn get_value(&mut self) -> Value {
        let mut token = self.get_current_token();
        let mut values: Vec<Value> = Vec::new();
        let mut buf: Vec<Value> = Vec::new();
        loop {
            match token {
                Tokens::Space => {
                    if buf.len() == 0 { continue; }
                    values.push(Value::Values(buf));
                    buf = Vec::new();
                },
                Tokens::CommandEnd(_) => break,
                Tokens::Literal(str) => buf.push(Value::Literal(str.clone())),
                Tokens::ExportSet => panic!("Unexpected token EXPORT_SET (=)"),
                Tokens::FileRead => buf.push(Value::Literal(token.to_str())),
                Tokens::Function => buf.push(Value::Literal(token.to_str())),
                Tokens::FileWrite => buf.push(Value::Literal(token.to_str())),
                Tokens::RedirectInto => panic!("Unexpected token REDIRECT (|)"),
                Tokens::FunctionCallEnd => panic!("Unexpected token FUNCTION CALL END ())"),
                Tokens::ArrayFunction(_) => panic!("Unexpected array function"),
                Tokens::StringFunction(_) => panic!("Unexpected string function"),
                Tokens::SubStart => /* SUB START PROCESS */,
                Tokens::SubEnd => panic!("Unexpected token SUB END ())"),
                Tokens::Else => buf.push(Value::Literal(token.to_str())),
                Tokens::End => buf.push(Value::Literal(token.to_str())),
                Tokens::For => return Expression::ForExpression(self.parse_for()),
                Tokens::If => return Expression::IfExpression(self.parse_if()),
                Tokens::Let => return Expression::LetExpression(self.parse_let()),
                Tokens::While => return Expression::WhileExpression(self.parse_while()),
                Tokens::StringVariable(_, _) => return Expression::Command(self.parse_call()),
                Tokens::ArrayVariable(_, _) => panic!("Unexpected array variable")
            }
            token = self.next();
        }
    }

    fn get_expression(&mut self, is_sub: bool) -> Expression {
        let mut token = self.get_current_token();
        loop {
            match token {
                Tokens::Space => continue,
                Tokens::CommandEnd(_) => continue,
                Tokens::Literal(_) => return Expression::Command(self.parse_call()),
                Tokens::ExportSet => panic!("Unexpected token EXPORT_SET (=)"),
                Tokens::FileRead => return Expression::FileSourceExpression(self.parse_read()),
                Tokens::Function => return Expression::Function(self.parse_function()),
                Tokens::FileWrite => return Expression::FileTargetExpression(self.parse_write(None)),
                Tokens::RedirectInto => panic!("Unexpected token REDIRECT (|)"),
                Tokens::FunctionCallEnd => panic!("Unexpected token FUNCTION CALL END ())"),
                Tokens::ArrayFunction(_) => panic!("Unexpected array function"),
                Tokens::StringFunction(_) => panic!("Unexpected string function"),
                Tokens::SubStart => return Expression::Command(self.parse_call()),
                Tokens::SubEnd => if !is_sub { panic!("Unexpected token SUB END ())") },
                Tokens::Else => panic!("Unexpected token ELSE"),
                Tokens::End => panic!("Unexpected token END"),
                Tokens::For => return Expression::ForExpression(self.parse_for()),
                Tokens::If => return Expression::IfExpression(self.parse_if()),
                Tokens::Let => return Expression::LetExpression(self.parse_let()),
                Tokens::While => return Expression::WhileExpression(self.parse_while()),
                Tokens::StringVariable(_, _) => return Expression::Command(self.parse_call()),
                Tokens::ArrayVariable(_, _) => panic!("Unexpected array variable")
            }
            token = self.next();
        }
    }

    fn next(&mut self) -> &Tokens {
        self.i += 1;
        self.get_current_token()
    }
    fn get_current_token(&self) -> &Tokens {
        self.tokens.get(self.i).unwrap()
    }
    fn get_next_token(&self) -> &Tokens {
        self.tokens.get(self.i + 1).unwrap()
    }
}

pub fn build_tree(tokens: Vec<Tokens>) -> Vec<Expression> {
    let mut expressions: Vec<Expression> = Vec::new();
    let mut tree = Tree { tokens, i: 0 };
    loop {
        if tree.i == tree.tokens.len() - 1 { break; }
        expressions.push(tree.get_expression());
    }

    expressions
}