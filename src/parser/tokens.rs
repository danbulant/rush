use std::io;

#[derive(Debug)]
pub enum Tokens {
    Space,
    Literal(String),
    Let,
    ExportSet,
    StringVariable(String, bool),
    ArrayVariable(String, bool),
    ArrayFunction(String),
    StringFunction(String),
    ParenthesisStart,
    ParenthesisEnd,
    CommandEnd(char),
    If,
    Else,
    While,
    For,
    Function,
    End,
    SubStart,
    RedirectInto,
    FileRead,
    FileWrite,
    And,
    Or,
    Break,
    JobCommandEnd
}

impl Tokens {
    fn detect(str: String) -> Tokens {
        match str.as_str() {
            "if" => Tokens::If,
            "while" => Tokens::While,
            "for" => Tokens::For,
            "let" => Tokens::Let,
            " " => Tokens::Space,
            "else" => Tokens::Else,
            "end" => Tokens::End,
            "$(" => Tokens::SubStart,
            "(" => Tokens::ParenthesisStart,
            ")" => Tokens::ParenthesisEnd,
            ">" => Tokens::FileWrite,
            "<" => Tokens::FileRead,
            "|" => Tokens::RedirectInto,
            "\r\n" | "\n" | ";" => Tokens::CommandEnd(str.chars().nth(0).unwrap()),
            "=" => Tokens::ExportSet,
            "break" => Tokens::Break,
            _ => Tokens::Literal(str)
        }
    }

    pub(crate) fn to_str(&self) -> String {
        match self {
            Tokens::Space => " ".to_string(),
            Tokens::Literal(str) => str.clone(),
            Tokens::Let => "let".to_string(),
            Tokens::StringVariable(str, bool) => format!("${}{}{}", match bool { true => "{", false => ""}, str.as_str(), match bool { true => "{", false => "" }),
            Tokens::ArrayVariable(str, bool) => format!("@{}{}{}", match bool { true => "{", false => ""}, str.as_str(), match bool { true => "{", false => "" }),
            Tokens::ArrayFunction(str) => format!("@{}", str.as_str()),
            Tokens::StringFunction(str) => format!("${}", str.as_str()),
            Tokens::CommandEnd(str) => str.to_string(),
            Tokens::ExportSet => "=".to_string(),
            Tokens::Function => "function".to_string(),
            Tokens::If => "if".to_string(),
            Tokens::Else => "else".to_string(),
            Tokens::While => "while".to_string(),
            Tokens::For => "for".to_string(),
            Tokens::End => "end".to_string(),
            Tokens::SubStart => "$(".to_string(),
            Tokens::ParenthesisStart => "(".to_string(),
            Tokens::ParenthesisEnd => ")".to_string(),
            Tokens::RedirectInto => "|".to_string(),
            Tokens::FileRead => "<".to_string(),
            Tokens::FileWrite => ">".to_string(),
            Tokens::And => "&&".to_string(),
            Tokens::Or => "||".to_string(),
            Tokens::Break => "break".to_string(),
            Tokens::JobCommandEnd => "&".to_string()
        }
    }
}


fn read_var_ahead(i: usize, text: &String) -> (usize, Tokens) {
    let mut x = i;
    let mut buf = String::new();
    let parens_mode = text.chars().nth(x + 1).unwrap() == '{';
    loop {
        x += 1;
        let letter: char = text.chars().nth(x).unwrap();
        match letter {
            'a'..='z' | 'A'..='Z' | '0'..='9' | ':' | '_' => {
                buf.push(letter.clone());
            }
            '}' => {
                if parens_mode {
                    x += 1;
                }
                break;
            }
            '?' => {
                buf.push(letter.clone());
                x += 1;
                break;
            }
            _ => { if !parens_mode { break } else { panic!("Invalid variable name") } }
        }
    }
    let token = match text.chars().nth(i).unwrap() {
        '$' => Tokens::StringVariable(buf, parens_mode),
        '@' => Tokens::ArrayVariable(buf, parens_mode),
        a => panic!("Invalid value {}", a)
    };
    (x - i - 1, token)
}

pub fn tokenize(reader: &mut dyn std::io::BufRead) -> io::Result<Vec<Tokens>> {
    let mut quote_active = false;
    let mut double_quote_active = false;
    let mut escape_active = false;
    let mut text = String::new();
    reader.read_to_string(&mut text);
    let mut text_length = text.len();

    let mut tokens: Vec<Tokens> = Vec::new();

    fn save_buf(buf: &mut String, tokens: &mut Vec<Tokens>) {
        if buf.len() > 0 { tokens.push(Tokens::detect(std::mem::take(buf))) }
    }

    let mut buf = String::new();
    let mut skipper = 0;
    for i in 0..text_length {
        if skipper > 0 {
            skipper -= 1;
            continue;
        }
        let letter: &char = &text.chars().nth(i).unwrap();
        let mut buf_add = true;
        match letter {
            '"' => if !escape_active && !quote_active { double_quote_active = !double_quote_active; buf_add = false },
            '\'' => if !escape_active && !double_quote_active { quote_active = !quote_active; buf_add = false },
            '$' => if !escape_active && !quote_active {
                save_buf(&mut buf, &mut tokens);
                if text_length > i && text.chars().nth(i + 1).unwrap() == '(' {
                    tokens.push(Tokens::SubStart);
                    skipper = 1;
                    buf_add = false;
                } else {
                    let (skippers, mut token) = read_var_ahead(i, &text);
                    match token {
                        Tokens::StringVariable(ref str, bool) => if !bool && !double_quote_active {
                            if text.len() > i + skippers && text.chars().nth(i + skippers).unwrap() == '(' {
                                token = Tokens::StringFunction(str.clone());
                            }
                        },
                        Tokens::ArrayVariable(ref str, bool) => if !bool && !double_quote_active {
                            if text.len() > i + skippers && text.chars().nth(i + skippers).unwrap() == '(' {
                                token = Tokens::ArrayFunction(str.clone());
                            }
                        }
                        _ => panic!("Cannot happen")
                    }
                    tokens.push(token);
                    skipper = skippers;
                    buf_add = false;
                }
            },
            ' ' => if !escape_active && !quote_active && !double_quote_active {
                save_buf(&mut buf, &mut tokens);
                tokens.push(Tokens::Space);
                let mut x = i;
                while text.chars().nth(x).unwrap() == ' ' {
                    x += 1;
                }
                skipper = x - i - 1;
                buf_add = false;
            },
            '(' => if !quote_active && !double_quote_active && !escape_active {
                save_buf(&mut buf, &mut tokens);
                tokens.push(Tokens::ParenthesisStart);
                buf_add = false;
            }
            ')' => if !quote_active && !double_quote_active && !escape_active {
                save_buf(&mut buf, &mut tokens);
                tokens.push(Tokens::ParenthesisEnd);
                buf_add = false;
            },
            '\\' => if !escape_active {
                escape_active = true;
                buf_add = false;
            } else {
                escape_active = false;
            },
            '=' => if !escape_active && !quote_active && !double_quote_active {
                save_buf(&mut buf, &mut tokens);
                tokens.push(Tokens::ExportSet);
                buf_add = false;
            },
            _ => {}
        }
        if letter.clone() != '\\' { escape_active = false; }
        if buf_add {
            buf.push(*letter);
        }
    }
    if buf.len() > 0 {
        tokens.push(Tokens::Literal(buf));
    }

    Ok(tokens)
}
