use chumsky::{error::{EmptyErr, Rich, Simple}, prelude::{any, choice, end, just, none_of, one_of, recursive, Recursive}, text, IterParser, Parser};


#[derive(Debug, Clone)]
pub struct Index {
    value: Box<Value>,
    index: Box<Value>
}

#[derive(Debug, Clone)]
pub enum Primitive {
    Number(f64),
    String(String),
    Variable(String),
    Index(Index)
}

#[derive(Debug, Clone)]
pub enum Bindable {
    Primitive(Primitive)
}

#[derive(Debug, Clone)]
pub struct Command {
    name: Box<Value>,
    args: Vec<Value>
}

#[derive(Debug, Clone)]
pub struct Set {
    name: Box<Bindable>,
    value: Box<Value>
}

#[derive(Debug, Clone)]
pub struct If {
    condition: Box<Command>,
    body: Vec<Statement>,
    else_body: Option<Vec<Statement>>
}

#[derive(Debug, Clone)]
pub struct While {
    condition: Box<Command>,
    body: Vec<Statement>,
    else_body: Option<Vec<Statement>>
}

#[derive(Debug, Clone)]
pub struct For {
    name: Box<Bindable>,
    iterable: Box<Value>,
    body: Vec<Statement>,
    else_body: Option<Vec<Statement>>
}

#[derive(Debug, Clone)]
pub struct Loop {
    body: Vec<Statement>
}

#[derive(Debug, Clone)]
pub struct Function {
    name: String,
    args: Vec<Bindable>,
    body: Vec<Statement>
}

#[derive(Debug, Clone)]
pub enum Statement {
    Command(Command),
    Set(Set),
    For(For),
    While(While),
    If(If),
    Loop(Loop),
    Function(Function),
    Return(Option<Value>),
    Break,
    Continue,
}

#[derive(Debug, Clone)]
pub enum Value {
    Primitive(Primitive),
    Group(Vec<Statement>)
}

pub fn parse<'a>() -> impl Parser<'a, &'a str, Vec<Statement>, chumsky::extra::Default> {
    // let ident = text::ident::<&'a str, chumsky::extra::Default>();
    let digits = text::digits(10).to_slice();

    let frac = just('.').then(digits);

    let exp = just('e')
        .or(just('E'))
        .then(one_of("+-").or_not())
        .then(digits);

    let number = just('-')
        .or_not()
        .then(text::int(10))
        .then(frac.or_not())
        .then(exp.or_not())
        .to_slice()
        .map(|s: &str| s.parse().unwrap())
        .boxed();
    // let op = |c| just(c).padded();

    let escape = just('\\')
        .then(choice((
            just('\\'),
            just('/'),
            just('"'),
            just('b').to('\x08'),
            just('f').to('\x0C'),
            just('n').to('\n'),
            just('r').to('\r'),
            just('t').to('\t'),
            just('u').ignore_then(text::digits(16).exactly(4).to_slice().validate(
                |digits, e, emitter| {
                    char::from_u32(u32::from_str_radix(digits, 16).unwrap()).unwrap_or_else(
                        || {
                            // emitter.emit(Rich::custom(e.span(), "invalid unicode character"));
                            '\u{FFFD}' // unicode replacement character
                        },
                    )
                },
            )),
        )))
        .ignored()
        .boxed();

    let direct_string = none_of("$()[]{}\\\"\n;")
        .and_is(text::whitespace().at_least(1).not())
        .ignored()
        .or(escape.clone())
        .repeated()
        .at_least(1)
        .to_slice()
        .map(ToString::to_string)
        .boxed();

    let delimited_string = none_of("\\\"")
        .ignored()
        .or(escape)
        .repeated()
        .to_slice()
        .map(ToString::to_string)
        .delimited_by(just('"'), just('"'))
        .boxed();

    let string = choice((
        delimited_string.clone(),
        direct_string.clone(),
    ));
    let eol = one_of("\n\r;");

    let variable = just('$').ignore_then(text::ident());

    let comment = just('#').then(any().and_is(just('\n').not()).repeated());

    let empty = text::inline_whitespace().ignored()
        .or(comment.ignored())
        .or(eol.ignored());

    recursive(|expr| {
        let primitive = choice((
            number.map(Primitive::Number),
            variable.map(|s: &str| Primitive::Variable(s.to_string())),
            string.map(Primitive::String),
        ));

        let group = expr.clone()
            .delimited_by(just('('), just(')'))
            .map(|v| Value::Group(v));

        let value = choice((
            group,
            primitive.clone().map(Value::Primitive),
            // index.map(|i| Value::Primitive(Primitive::Index(i))),
        ));

        let index = value.clone()
            .foldl(
            value
                .clone()
                .padded_by(text::inline_whitespace())
                .delimited_by(just('['), just(']'))
                .repeated(),
                |value, index| Value::Primitive(Primitive::Index(Index {
                value: Box::new(value.clone()),
                index: Box::new(index)
            })));

        let value = choice((
            index,
            value,
        ));
        // let value = recursive(|newvalue: Recursive<dyn Parser<'_, &str, Value>>| {

        //     value
        // });
        
        let bindable = primitive.clone().map(Bindable::Primitive);

        let bindable_group = bindable
            .clone()
            .padded()
            .separated_by(just(","))
            .collect()
            .delimited_by(just('('), just(')'));
        
        let block = expr.clone()
            .or(empty.to(vec![]))
            .delimited_by(just('{'), just('}'));

        let cmdname = value.clone()
            .and_is(choice((
                just("set"),
                just("if"),
                just("while"),
                just("for"),
                just("loop"),
                just("break"),
                just("continue"),
                just("return"),
                just("fn"),
            )).then(end()).not());

        let args = value.clone()
            .separated_by(text::inline_whitespace().at_least(1))
            .allow_leading()
            .allow_trailing()
            .collect();

        let command = 
            cmdname
            .padded_by(text::inline_whitespace())
            .then(args)
            .map(|(name, args): (Value, Vec<Value>)| {
                Command {
                    name: Box::new(name),
                    args: args.into_iter().map(|v| v.clone()).collect()
                }
            });

        let set = just("set")
            .then_ignore(text::inline_whitespace().at_least(1))
            .ignore_then(bindable.clone())
            .then_ignore(just('=').padded_by(text::inline_whitespace()))
            .then(value.clone())
            .map(|(name, value): (Bindable, Value)| {
                Set {
                    name: Box::new(name),
                    value: Box::new(value)
                }
            });

        let else_ = just("else")
            .then(text::inline_whitespace().at_least(1))
            .ignore_then(choice((
                block.clone(),
                expr.clone()
            )));

        let if_ = just("if")
            .then_ignore(text::inline_whitespace().at_least(1))
            .ignore_then(command.clone())
            .then(block.clone().padded_by(text::inline_whitespace()))
            .then(else_.clone().or_not())
            .map(|((cond, body), else_body): ((Command, Vec<Statement>), _)| {
                If {
                    condition: Box::new(cond),
                    body,
                    else_body
                }
            });

        let while_ = just("while")
            .then_ignore(text::inline_whitespace().at_least(1))
            .ignore_then(command.clone())
            .then(block.clone().padded_by(text::inline_whitespace()))
            .then(else_.clone().or_not())
            .map(|((cond, body), else_body): ((Command, Vec<Statement>), _)| {
                While {
                    condition: Box::new(cond),
                    body,
                    else_body
                }
            });

        let for_ = just("for")
            .then_ignore(text::inline_whitespace().at_least(1))
            .ignore_then(bindable.clone())
            .then_ignore(just("in").padded())
            .then(value.clone())
            .then(block.clone().padded_by(text::inline_whitespace()))
            .then(else_.clone().or_not())
            .map(|(((name, iterable), body), else_body): (((Bindable, Value), Vec<Statement>), _)| {
                For {
                    name: Box::new(name),
                    iterable: Box::new(iterable),
                    body,
                    else_body
                }
            });

        let loop_ = just("loop")
            .ignore_then(block.clone().padded_by(text::inline_whitespace()))
            .map(|body: Vec<Statement>| {
                Loop {
                    body
                }
            });

        let return_ = just("return")
            .then_ignore(text::inline_whitespace().at_least(1))
            .ignore_then(value.clone().or_not())
            .map(|v: Option<Value>| {
                Statement::Return(v)
            });

        let function = just("fn")
            .then_ignore(text::inline_whitespace().at_least(1))
            .ignore_then(text::ident())
            .then(bindable_group.clone())
            .then(block.clone().padded_by(text::inline_whitespace()))
            .map(|((name, args), body): ((&str, Vec<Bindable>), Vec<Statement>)| {
                Function {
                    name: name.to_string(),
                    args: args,
                    body
                }
            });

        let statement = choice((
            set.map(Statement::Set),
            if_.map(Statement::If),
            while_.map(Statement::While),
            for_.map(Statement::For),
            loop_.map(Statement::Loop),
            function.map(Statement::Function),
            return_,
            just("break").to(Statement::Break),
            just("continue").to(Statement::Continue),
            command.map(Statement::Command),
        ));

        statement
            .padded_by(text::inline_whitespace().ignored().or(comment.ignored()))
            .separated_by(eol.repeated().at_least(1))
            .at_least(1)
            .allow_trailing()
            .allow_leading()
            .collect()
    })
}