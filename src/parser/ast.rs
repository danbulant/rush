use crate::parser::vars::Variable;

#[derive(Debug)]
pub struct LetExpression {
    pub key: String,
    pub value: Value
}

#[derive(Debug)]
pub struct IfExpression {
    pub condition: Expression,
    pub contents: Vec<Expression>
}

#[derive(Debug)]
pub struct WhileExpression {
    pub condition: Expression,
    pub contents: Vec<Expression>
}

#[derive(Debug)]
pub struct ForExpression {
    pub key: String,
    pub list: Value,
    pub contents: Vec<Expression>
}

#[derive(Debug)]
pub enum ForValue {
    Value(Value),
    Range(Some(u32), Some(u32))
}

#[derive(Debug)]
pub enum Value {
    Literal(String),
    Variable(Variable),
    Expression(Expression)
}

#[derive(Debug)]
pub struct FunctionExpression {
    pub args: Vec<Value>,
    pub body: Expression
}

pub struct RedirectTargetExpression {

}

#[derive(Debug)]
pub enum Expression {
    LetExpression(LetExpression),
    Command(Vec<Value>),
    Function(FunctionExpression),
    IfExpression(IfExpression),
    WhileExpression(WhileExpression),
    ForExpression(ForExpression)
}