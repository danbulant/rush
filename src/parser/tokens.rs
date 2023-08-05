use anyhow::{Result, bail};

#[derive(Debug)]
pub struct Token {
    pub token: Tokens,
    pub start: usize,
    pub end: usize
}

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
    ArrayStart,
    ArrayEnd,
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
            "[" => Tokens::ArrayStart,
            "]" => Tokens::ArrayEnd,
            ">" => Tokens::FileWrite,
            "<" => Tokens::FileRead,
            "|" => Tokens::RedirectInto,
            "\r\n" | "\n" | ";" => Tokens::CommandEnd(str.chars().next().unwrap()),
            "&&" => Tokens::And,
            "||" => Tokens::Or,
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
            Tokens::ArrayStart => "[".to_string(),
            Tokens::ArrayEnd => "]".to_string(),
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


fn read_var_ahead(i: usize, text: &str) -> Result<(usize, Token)> {
    let mut x = i;
    let mut buf = String::new();
    let parens_mode = text.chars().nth(x + 1).unwrap() == '{';
    if parens_mode { x += 1 }
    loop {
        x += 1;
        let letter: char = text.chars().nth(x).unwrap();
        match letter {
            'a'..='z' | 'A'..='Z' | '0'..='9' | ':' | '_' => {
                buf.push(letter);
            }
            '}' => {
                if parens_mode {
                    x += 1;
                }
                break;
            }
            '?' => {
                buf.push(letter);
                x += 1;
                break;
            }
            l => { if !parens_mode { break } else { bail!("Invalid variable name (starting with '{}{}')", buf, l) } }
        }
    }
    let token = match text.chars().nth(i).unwrap() {
        '$' => Token { token: Tokens::StringVariable(buf, parens_mode), start: i, end: i + x },
        '@' => Token { token: Tokens::ArrayVariable(buf, parens_mode), start:i , end: i+x },
        a => bail!("Invalid value {}", a)
    };
    Ok((x - i - 1, token))
}

pub fn tokenize(reader: &mut dyn std::io::BufRead) -> Result<Vec<Token>> {
    let mut quote_active = false;
    let mut double_quote_active = false;
    let mut escape_active = false;
    let mut text = String::new();
    reader.read_to_string(&mut text)?;
    let text_length = text.len();

    let mut tokens: Vec<Token> = Vec::new();

    fn save_buf(buf: &mut String, tokens: &mut Vec<Token>, i: usize) {
        if !buf.is_empty() { tokens.push(Token { token: Tokens::detect(std::mem::take(buf)), end: i, start: i - buf.len() }) }
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
            '$' | '@' => if !escape_active && !quote_active {
                save_buf(&mut buf, &mut tokens, i);
                if *letter == '$' && text_length > i && text.chars().nth(i + 1).unwrap() == '(' {
                    tokens.push(Token { token: Tokens::SubStart, start: i, end: i+1 });
                    skipper = 1;
                    buf_add = false;
                } else {
                    let (mut skippers, mut token) = read_var_ahead(i, &text)?;
                    match token.token {
                        Tokens::StringVariable(ref str, bool) => if !bool && !double_quote_active && text.len() > i + skippers + 1 && text.chars().nth(i + skippers + 1).unwrap() == '(' {
                            skippers += 1;
                            token = Token { token: Tokens::StringFunction(str.clone()), end: i + skippers, start: i };
                        },
                        Tokens::ArrayVariable(ref str, bool) => if !bool && !double_quote_active && text.len() > i + skippers + 1 && text.chars().nth(i + skippers + 1).unwrap() == '(' {
                            skippers += 1;
                            token = Token { token: Tokens::ArrayFunction(str.clone()), end: i+skippers, start: i };
                        }
                        _ => bail!("Cannot happen")
                    }
                    tokens.push(token);
                    skipper = skippers;
                    buf_add = false;
                }
            },
            ';' | '\r' | '\n' => if !escape_active && !quote_active && !double_quote_active {
                save_buf(&mut buf, &mut tokens, i);
                tokens.push(Token { token: Tokens::CommandEnd(*letter), start: i, end: i });
                let mut x = 0;
                while x < text.len() - 1 && matches!(text.chars().nth(x).unwrap(), '\n' | '\r' | ';' | ' ') {
                    x += 1;
                }
                if x > 0 {
                    skipper = x - 1;
                }
                buf_add = false;
            },
            '&' => if !escape_active && !quote_active && !double_quote_active {
                save_buf(&mut buf, &mut tokens, i);
                if i + 1 < text.len() && text.chars().nth(i+1).unwrap() == '&' {
                    tokens.push(Token { token: Tokens::And, start: i, end: i+1 });
                    skipper = 1;
                } else {
                    tokens.push(Token { token: Tokens::JobCommandEnd, start: i , end: i });
                }
                buf_add = false;
            },
            '|' => if !escape_active && !quote_active && !double_quote_active {
                save_buf(&mut buf, &mut tokens, i);
                if i + 1 < text.len() && text.chars().nth(i+1).unwrap() == '|' {
                    tokens.push(Token { token: Tokens::Or, start: i, end: i+1 });
                    skipper = 1;
                } else {
                    tokens.push(Token { token: Tokens::RedirectInto, start: i, end: i });
                }
                buf_add = false;
            },
            ' ' => if !escape_active && !quote_active && !double_quote_active {
                save_buf(&mut buf, &mut tokens, i);
                tokens.push(Token { token: Tokens::Space, start: i, end: i });
                let mut x = i;
                while text.chars().nth(x).unwrap() == ' ' {
                    x += 1;
                }
                skipper = x - i - 1;
                buf_add = false;
            },
            '(' => if !quote_active && !double_quote_active && !escape_active {
                save_buf(&mut buf, &mut tokens, i);
                tokens.push(Token { token: Tokens::ParenthesisStart, start: i, end: i });
                buf_add = false;
            }
            ')' => if !quote_active && !double_quote_active && !escape_active {
                save_buf(&mut buf, &mut tokens, i);
                tokens.push(Token { token: Tokens::ParenthesisEnd, start: i, end: i });
                buf_add = false;
            },
            '[' => if !quote_active && !double_quote_active && !escape_active {
                save_buf(&mut buf, &mut tokens, i);
                tokens.push(Token { token: Tokens::ArrayStart, start: i, end: i });
                buf_add = false;
            },
            ']' => if !quote_active && !double_quote_active && !escape_active {
                save_buf(&mut buf, &mut tokens, i);
                tokens.push(Token { token: Tokens::ArrayEnd, start: i, end: i });
                buf_add = false;
            },
            '\\' => if !escape_active {
                escape_active = true;
                buf_add = false;
            } else {
                escape_active = false;
            },
            '=' => if !escape_active && !quote_active && !double_quote_active {
                save_buf(&mut buf, &mut tokens, i);
                tokens.push(Token { token: Tokens::ExportSet, start: i, end: i });
                buf_add = false;
            },
            '#' => if !escape_active && !quote_active && !double_quote_active {
                save_buf(&mut buf, &mut tokens, i);
                buf_add = false;
                let mut x = 0;
                while x + i + 1 < text.len() && text.chars().nth(x + i + 1).unwrap() != '\n' {
                    x += 1;
                }
                skipper = x;
            }
            _ => {}
        }
        if *letter != '\\' { escape_active = false; }
        if buf_add {
            buf.push(*letter);
        }
    }
    save_buf(&mut buf, &mut tokens, text.len());

    Ok(tokens)
}
