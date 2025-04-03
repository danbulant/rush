use chumsky::{error::{EmptyErr, Rich, Simple}, prelude::{choice, just, none_of, one_of, recursive}, text, IterParser, Parser};


#[derive(Debug, Clone)]
struct Index {
    value: Box<Value>,
    index: Box<Value>
}

#[derive(Debug, Clone)]
enum Primitive {
    Number(f64),
    String(String),
    Variable(String),
    Index(Index)
}

#[derive(Debug, Clone)]
enum Bindable {
    Primitive(Primitive),
}

#[derive(Debug, Clone)]
struct Command {
    name: Box<Value>,
    args: Vec<Value>
}

#[derive(Debug, Clone)]
struct Set {
    name: Box<Bindable>,
    value: Box<Value>
}

#[derive(Debug, Clone)]
struct If {
    condition: Box<Command>,
    body: Vec<Statement>,
    else_body: Option<Vec<Statement>>
}

#[derive(Debug, Clone)]
struct While {
    condition: Box<Command>,
    body: Vec<Statement>,
    else_body: Option<Vec<Statement>>
}

#[derive(Debug, Clone)]
struct For {
    name: Box<Bindable>,
    iterable: Box<Value>,
    body: Vec<Statement>,
    else_body: Option<Vec<Statement>>
}

#[derive(Debug, Clone)]
struct Loop {
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
    Break
}

#[derive(Debug, Clone)]
enum Value {
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
    // let r#break = just("break").map(|_| Statement::Break);

    let variable = just('$').ignore_then(text::ident());

    recursive(|expr| {
        let primitive = choice((
            number.map(Primitive::Number),
            variable.map(|s: &str| Primitive::Variable(s.to_string())),
            string.map(Primitive::String),
        ));

        let bindable = primitive.clone();

        let group = just('(')
            .ignore_then(expr.clone())
            .then_ignore(just(')'))
            .map(|v| Value::Group(v));

        let value = choice((
            group,
            primitive.map(Value::Primitive),
        ));

        let command = value
            .separated_by(text::whitespace().at_least(1))
            .at_least(1)
            .allow_trailing()
            .allow_leading()
            .collect()
            .map(|args: Vec<Value>| {
                let name = args[0].clone();
                let args = args[1..].to_vec();

                Command {
                    name: Box::new(name),
                    args: args.into_iter().map(|v| v.clone()).collect()
                }
            });

        let statement = command.map(Statement::Command);

        statement.separated_by(eol).at_least(1).collect()
    })
}