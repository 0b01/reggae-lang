use std::str::Chars;
use std::iter::Peekable;
use std::ops::DerefMut;

/// Represents a primitive syntax token.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Binary,
    Bang,
    Comma,
    Colon,
    Comment,
    Fn,
    Else,
    EOF,
    Extern,
    For,
    Ident(String),
    If,
    In,
    Number(f64),
    Op(char),
    LBrace,
    RBrace,
    LParen,
    RParen,
    Then,
    Mot,
    Lru,
    Mru,
    Lfu,
    Mfu,
    Unary,
    Var,
    Str(String),
}

/// Defines an error encountered by the `Lexer`.
pub struct LexError {
    pub error: &'static str,
    pub index: usize
}

impl LexError {
    pub fn new(msg: &'static str) -> LexError {
        LexError { error: msg, index: 0 }
    }

    pub fn with_index(msg: &'static str, index: usize) -> LexError {
        LexError { error: msg, index: index }
    }
}

/// Defines the result of a lexing operation; namely a
/// `Token` on success, or a `LexError` on failure.
pub type LexResult = Result<Token, LexError>;

/// Defines a lexer which transforms an input `String` into
/// a `Token` stream.
pub struct Lexer<'a> {
    input: &'a str,
    chars: Box<Peekable<Chars<'a>>>,
    pos: usize
}

impl<'a> Lexer<'a> {
    /// Creates a new `Lexer`, given its source `input`.
    pub fn new(input: &'a str) -> Lexer<'a> {
        Lexer { input: input, chars: Box::new(input.chars().peekable()), pos: 0 }
    }

    /// Lexes and returns the next `Token` from the source code.
    pub fn lex(&mut self) -> LexResult {
        let chars = self.chars.deref_mut();
        let src = self.input;

        let mut pos = self.pos;

        // Skip whitespaces
        loop {
            // Note: the following lines are in their own scope to
            // limit how long 'chars' is borrowed, and in order to allow
            // it to be borrowed again in the loop by 'chars.next()'.
            {
                let ch = chars.peek();

                if ch.is_none() {
                    self.pos = pos;

                    return Ok(Token::EOF);
                }

                if !ch.unwrap().is_whitespace() {
                    break;
                }
            }

            chars.next();
            pos += 1;
        }

        let start = pos;
        let next = chars.next();

        if next.is_none() {
            return Ok(Token::EOF);
        }

        pos += 1;

        // Actually get the next token.
        let result = match next.unwrap() {
            '(' => Ok(Token::LParen),
            ')' => Ok(Token::RParen),
            '{' => Ok(Token::LBrace),
            '}' => Ok(Token::RBrace),
            ',' => Ok(Token::Comma),
            '!' => Ok(Token::Bang),
            ':' => Ok(Token::Colon),
            '"' => {
                let mut value = String::new();

                while let Ok(ch) = self.read_escaped_char() {
                    if ch != '"' {
                        value.push(ch);
                    } else if ch == '"' {
                        return Ok(Token::Str(value));
                    }
                }
                Err(LexError::new("unclosed string"))
            },

            '.' | '0' ..= '9' => {
                // Parse number literal
                loop {
                    let ch = match chars.peek() {
                        Some(ch) => *ch,
                        None => return Ok(Token::EOF)
                    };

                    // Parse float.
                    if ch != '.' && !ch.is_digit(16) {
                        break;
                    }

                    chars.next();
                    pos += 1;
                }

                Ok(Token::Number(src[start..pos].parse().unwrap()))
            },


            '/' => {
                // Comment
                if let Some('/') = chars.peek() {
                    loop {
                        let ch = chars.next();
                        pos += 1;
                        if ch == Some('\n') {
                            break;
                        }
                    }
                    Ok(Token::Comment)
                } else if let Some('*') = chars.peek() {
                    loop {
                        let ch = chars.next();
                        pos += 1;
                        if ch == Some('*') {
                            if let Some('/') = chars.peek() {
                                let _ = chars.next();
                                break;
                            }
                        }
                    }
                    Ok(Token::Comment)
                } else {
                    Ok(Token::Op('/'))
                }

            },

            'a' ..= 'z' | 'A' ..= 'Z' | '_' => {
                // Parse identifier
                loop {
                    let ch = match chars.peek() {
                        Some(ch) => *ch,
                        None => return Ok(Token::EOF)
                    };

                    // A word-like identifier only contains underscores and alphanumeric characters.
                    if ch != '_' && ch != ',' && !ch.is_alphanumeric() {
                        break;
                    }

                    chars.next();
                    pos += 1;
                }

                match &src[start..pos] {
                    "fn" => Ok(Token::Fn),
                    "extern" => Ok(Token::Extern),
                    "if" => Ok(Token::If),
                    "then" => Ok(Token::Then),
                    "else" => Ok(Token::Else),
                    "for" => Ok(Token::For),
                    "in" => Ok(Token::In),
                    "unary" => Ok(Token::Unary),
                    "binary" => Ok(Token::Binary),
                    "var" => Ok(Token::Var),
                    "mot" => Ok(Token::Mot),
                    "lru" => Ok(Token::Lru),
                    "mru" => Ok(Token::Mru),
                    "lfu" => Ok(Token::Lfu),
                    "mfu" => Ok(Token::Mfu),

                    ident => Ok(Token::Ident(ident.to_string()))
                }
            },

            op => {
                // Parse operator
                Ok(Token::Op(op))
            }
        };

        // Update stored position, and return
        self.pos = pos;

        result
    }

    fn read_escaped_char(&mut self) -> Result<char, LexError> {
        if let Some(ch) = self.chars.next() {
            if ch == '\\' {
                let ch = self.chars.next().ok_or(LexError::new("no input"))?;

                match ch {
                    '\\' => Ok('\\'),
                    'n' => Ok('\n'),
                    't' => Ok('\t'),
                    'r' => Ok('\r'),
                    '\"' => Ok('\"'),
                    '\'' => Ok('\''),
                    '0' => Ok('\0'),

                    'e' => unimplemented!(),
                    'v' => unimplemented!(),
                    'x' => unimplemented!(),
                    'u' => unimplemented!(),

                    _ => {
                        Err(LexError::new("unknown escape char"))
                    }
                }
            } else {
                Ok(ch)
            }
        } else {
            Err(LexError::new("no input"))
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    /// Lexes the next `Token` and returns it.
    /// On EOF or failure, `None` will be returned.
    fn next(&mut self) -> Option<Self::Item> {
        match self.lex() {
            Ok(Token::EOF) | Err(_) => None,
            Ok(token) => Some(token)
        }
    }
}

