use crate::lex::{Token, Lexer};
use std::collections::HashMap;
use Token::*;

const ANONYMOUS_FUNCTION_NAME: &str = "anonymous";

#[derive(Debug)]
pub enum Cache {
    Lru(isize),
    Mru(isize),
    Lfu(isize),
    Mfu(isize),
    None,
}

/// Defines a primitive expression.
#[derive(Debug)]
pub enum Expr {
    Binary {
        op: char,
        left: Box<Expr>,
        right: Box<Expr>
    },

    Call {
        fn_name: String,
        args: Vec<Expr>,
        bang: bool,
    },

    Conditional {
        cond: Box<Expr>,
        consequence: Box<Expr>,
        alternative: Box<Expr>,
    },

    For {
        var_name: String,
        start: Box<Expr>,
        end: Box<Expr>,
        step: Option<Box<Expr>>,
        body: Box<Expr>
    },

    Number(f64),
    Str(String),

    Variable(String),

    VarIn {
        variables: Vec<(String, Option<Expr>)>,
        body: Box<Expr>
    }
}

/// Defines the prototype (name and parameters) of a function.
#[derive(Debug)]
pub struct Prototype {
    pub name: String,
    pub args: Vec<(String, String)>,
    pub is_op: bool,
    pub prec: usize,
}

/// Defines a user-defined or external function.
#[derive(Debug)]
pub struct Function {
    pub prototype: Prototype,
    pub body: Option<Expr>,
    pub is_anon: bool,
    pub cache: Cache,
}

/// Represents the `Expr` parser.
pub struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    prec: &'a mut HashMap<char, i32>
}

// I'm ignoring the 'must_use' lint in order to call 'self.advance' without checking
// the result when an EOF is acceptable.
#[allow(unused_must_use)]
impl<'a> Parser<'a> {
    /// Creates a new parser, given an input `str` and a `HashMap` binding
    /// an operator and its precedence in binary expressions.
    pub fn new(input: String, op_precedence: &'a mut HashMap<char, i32>) -> Self {
        let mut lexer = Lexer::new(input.as_str());
        let tokens = lexer.by_ref().collect();

        Parser {
            tokens: tokens,
            prec: op_precedence,
            pos: 0
        }
    }

    /// Parses the content of the parser.
    pub fn parse(&mut self) -> Result<Function, String> {
        let result = match self.current()? {
            Fn | Lru | Mru | Lfu | Mfu => self.parse_def(),
            Extern => self.parse_extern(),
            _ => self.parse_toplevel_expr()
        };

        match result {
            Ok(result) => {
                if !self.at_end() {
                    Err("Unexpected token after parsed expression.".to_owned())
                } else {
                    Ok(result)
                }
            },

            err => err
        }
    }

    fn eat(&mut self, token: Token) -> Result<(), String> {
        if self.curr() == token {
            self.advance()
        } else {
            Err(format!("Expecting {:?} but found {:?}.", token, self.curr()))
        }
    }

    /// Returns the current `Token`, without performing safety checks beforehand.
    fn curr(&self) -> Token {
        self.tokens[self.pos].clone()
    }

    /// Returns the current `Token`, or an error that
    /// indicates that the end of the file has been unexpectedly reached if it is the case.
    fn current(&self) -> Result<Token, String> {
        if self.pos >= self.tokens.len() {
            Err("Unexpected end of file.".to_owned())
        } else {
            Ok(self.tokens[self.pos].clone())
        }
    }

    /// Advances the position, and returns an empty `Result` whose error
    /// indicates that the end of the file has been unexpectedly reached.
    /// This allows to use the `self.advance()?;` syntax.
    fn advance(&mut self) -> Result<(), String> {
        let npos = self.pos + 1;

        self.pos = npos;

        if npos < self.tokens.len() {
            Ok(())
        } else {
            Err("Unexpected end of file.".to_owned())
        }
    }

    /// Returns a value indicating whether or not the `Parser`
    /// has reached the end of the input.
    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    /// Returns the precedence of the current `Token`, or 0 if it is not recognized as a binary operator.
    fn get_tok_precedence(&self) -> i32 {
        if let Ok(Token::Op(op)) = self.current() {
            *self.prec.get(&op).unwrap_or(&100)
        } else {
            -1
        }
    }

    /// Parses the prototype of a function, whether external or user-defined.
    fn parse_prototype(&mut self) -> Result<Prototype, String> {
        let (id, is_operator, precedence) = match self.curr() {
            Ident(id) => {
                self.advance()?;

                (id, false, 0)
            },

            Binary => {
                self.advance()?;

                let op = match self.curr() {
                    Op(ch) => ch,
                    _ => return Err("Expected operator in custom operator declaration.".to_owned())
                };

                self.advance()?;

                let mut name = String::from("binary");

                name.push(op);

                let prec = if let Number(prec) = self.curr() {
                    self.advance()?;

                    prec as usize
                } else {
                    0
                };

                self.prec.insert(op, prec as i32);

                (name, true, prec)
            },

            Unary => {
                self.advance()?;

                let op = match self.curr() {
                    Op(ch) => ch,
                    _ => return Err("Expected operator in custom operator declaration.".to_owned())
                };

                let mut name = String::from("unary");

                name.push(op);

                self.advance()?;

                (name, true, 0)
            },

            _ => return Err("Expected identifier in prototype declaration.".to_owned())
        };

        self.eat(LParen);

        if let RParen = self.curr() {
            self.advance();

            return Ok(Prototype {
                name: id,
                args: vec![],
                is_op: is_operator,
                prec: precedence
            });
        }

        let mut args = vec![];

        loop {
            match self.curr() {
                Ident(name) => {
                    self.advance();
                    self.eat(Token::Colon);
                    match self.curr() {
                        Ident(ty) => {
                            args.push((name, ty));
                        }
                        _ => return Err("Expected type".to_owned())
                    }
                }
                _ => return Err("Expected identifier in parameter declaration.".to_owned())
            }

            self.advance()?;

            match self.curr() {
                RParen => {
                    self.advance();
                    break;
                },
                Comma => {
                    self.advance();
                },
                _ => return Err("Expected ',' or ')' character in prototype declaration.".to_owned())
            }
        }

        Ok(Prototype {
            name: id,
            args,
            is_op: is_operator,
            prec: precedence
        })
    }

    /// Parses a user-defined function.
    fn parse_def(&mut self) -> Result<Function, String> {
        let cache = match self.curr() {
            Lru => Cache::Lru({
                self.advance()?; if Token::Bang == self.curr() {self.advance(); self.parse_number()?} else {-1}
            }),
            Mru => Cache::Mru({
                self.advance()?; if Token::Bang == self.curr() {self.advance(); self.parse_number()?} else {-1}
            }),
            Lfu => Cache::Lfu({
                self.advance()?; if Token::Bang == self.curr() {self.advance(); self.parse_number()?} else {-1}
            }),
            Mfu => Cache::Mfu({
                self.advance()?; if Token::Bang == self.curr() {self.advance(); self.parse_number()?} else {-1}
            }),
            Fn => { Cache::Lru(-1)}
            _ => return Err("Wrong function decl keyword".to_owned()),
        };

        self.advance()?;

        // Parse signature of function
        let proto = self.parse_prototype()?;
        self.eat(LBrace);

        // Parse body of function
        let body = self.parse_expr()?;

        self.eat(RBrace);

        // Return new function
        Ok(Function {
            prototype: proto,
            body: Some(body),
            is_anon: false,
            cache,
        })
    }

    /// Parses an external function declaration.
    fn parse_extern(&mut self) -> Result<Function, String> {
        // Eat 'extern' keyword
        self.pos += 1;

        // Parse signature of extern function
        let proto = self.parse_prototype()?;

        Ok(Function {
            prototype: proto,
            body: None,
            is_anon: false,
            cache: Cache::None,
        })
    }

    /// Parses any expression.
    fn parse_expr(&mut self) -> Result<Expr, String> {
        match self.parse_unary_expr() {
            Ok(left) => self.parse_binary_expr(0, left),
            err => err
        }
    }

    /// Parses a literal number.
    fn parse_number(&mut self) -> Result<isize, String> {
        // Simply convert Token::Number to Expr::Number
        match self.curr() {
            Number(nb) => {
                self.advance();
                Ok(nb as isize)
            },
            _ => Err("Expected number literal.".to_owned())
        }
    }

    /// Parses a literal number.
    fn parse_nb_expr(&mut self) -> Result<Expr, String> {
        // Simply convert Token::Number to Expr::Number
        match self.curr() {
            Number(nb) => {
                self.advance();
                Ok(Expr::Number(nb))
            },
            _ => Err("Expected number literal.".to_owned())
        }
    }

    /// Parses an expression enclosed in parenthesis.
    fn parse_paren_expr(&mut self) -> Result<Expr, String> {
        match self.current()? {
            LParen => (),
            _ => return Err("Expected '(' character at start of parenthesized expression.".to_owned())
        }

        self.advance()?;

        let expr = self.parse_expr()?;

        match self.current()? {
            RParen => (),
            _ => return Err("Expected ')' character at end of parenthesized expression.".to_owned())
        }

        self.advance();

        Ok(expr)
    }

    /// Parses an expression that starts with an identifier (either a variable or a function call).
    fn parse_id_expr(&mut self) -> Result<Expr, String> {
        let id = match self.curr() {
            Ident(id) => id,
            _ => return Err("Expected identifier.".to_owned())
        };

        if self.advance().is_err() {
            return Ok(Expr::Variable(id));
        }

        match self.curr() {
            LParen => {
                self.advance()?;

                if let RParen = self.curr() {
                    self.advance()?;
                    let bang = Token::Bang == self.curr();
                    return Ok(Expr::Call { fn_name: id, args: vec![], bang });
                }

                let mut args = vec![];

                loop {
                    args.push(self.parse_expr()?);

                    match self.current()? {
                        Comma => (),
                        RParen => break,
                        _ => return Err("Expected ',' character in function call.".to_owned())
                    }

                    self.advance()?;
                }

                self.advance();

                let bang = Token::Bang == self.curr();

                Ok(Expr::Call { fn_name: id, args: args, bang })
            },

            _ => Ok(Expr::Variable(id))
        }
    }

    /// Parses an unary expression.
    fn parse_unary_expr(&mut self) -> Result<Expr, String> {
        let op = match self.current()? {
            Bang => {
                self.advance()?;
                '!'
            }
            Op(ch) => {
                self.advance()?;
                ch
            },
            _ => return self.parse_primary()
        };

        let mut name = String::from("unary");

        name.push(op);

        Ok(Expr::Call {
            fn_name: name,
            args: vec![ self.parse_unary_expr()? ],
            bang: false,
        })
    }

    /// Parses a binary expression, given its left-hand expression.
    fn parse_binary_expr(&mut self, prec: i32, mut left: Expr) -> Result<Expr, String> {
        loop {
            let curr_prec = self.get_tok_precedence();

            if curr_prec < prec || self.at_end() {
                return Ok(left);
            }

            let op = match self.curr() {
                Op(op) => op,
                _ => return Err("Invalid operator.".to_owned())
            };

            self.advance()?;

            let mut right = self.parse_unary_expr()?;

            let next_prec = self.get_tok_precedence();

            if curr_prec < next_prec {
                right = self.parse_binary_expr(curr_prec + 1, right)?;
            }

            left = Expr::Binary {
                op: op,
                left: Box::new(left),
                right: Box::new(right)
            };
        }
    }

    /// Parses a conditional if..then..else expression.
    fn parse_conditional_expr(&mut self) -> Result<Expr, String> {
        // eat 'if' token
        self.advance()?;

        let cond = self.parse_expr()?;

        // eat 'then' token
        match self.current() {
            Ok(Then) => self.advance()?,
            _ => return Err("Expected 'then' keyword.".to_owned())
        }

        let then = self.parse_expr()?;

        // eat 'else' token
        match self.current() {
            Ok(Else) => self.advance()?,
            _ => return Err("Expected 'else' keyword.".to_owned())
        }

        let otherwise = self.parse_expr()?;

        Ok(Expr::Conditional {
            cond: Box::new(cond),
            consequence: Box::new(then),
            alternative: Box::new(otherwise)
        })
    }

    /// Parses a loop for..in.. expression.
    fn parse_for_expr(&mut self) -> Result<Expr, String> {
        // eat 'for' token
        self.advance()?;

        let name = match self.curr() {
            Ident(n) => n,
            _ => return Err("Expected identifier in for loop.".to_owned())
        };

        // eat identifier
        self.advance()?;

        // eat '=' token
        match self.curr() {
            Op('=') => self.advance()?,
            _ => return Err("Expected '=' character in for loop.".to_owned())
        }

        let start = self.parse_expr()?;

        // eat ',' token
        match self.current()? {
            Comma => self.advance()?,
            _ => return Err("Expected ',' character in for loop.".to_owned())
        }

        let end = self.parse_expr()?;

        // parse (optional) step expression
        let step = match self.current()? {
            Comma => {
                self.advance()?;

                Some(self.parse_expr()?)
            },

            _ => None
        };

        // eat 'in' token
        match self.current()? {
            In => self.advance()?,
            _ => return Err("Expected 'in' keyword in for loop.".to_owned())
        }

        let body = self.parse_expr()?;

        Ok(Expr::For {
            var_name: name,
            start: Box::new(start),
            end: Box::new(end),
            step: step.map(Box::new),
            body: Box::new(body)
        })
    }

    /// Parses a var..in expression.
    fn parse_var_expr(&mut self) -> Result<Expr, String> {
        // eat 'var' token
        self.advance()?;

        let mut variables = Vec::new();

        // parse variables
        loop {
            let name = match self.curr() {
                Ident(name) => name,
                _ => return Err("Expected identifier in 'var..in' declaration.".to_owned())
            };

            self.advance()?;

            // read (optional) initializer
            let initializer = match self.curr() {
                Op('=') => Some({
                    self.advance()?;
                    self.parse_expr()?
                }),

                _ => None
            };

            variables.push((name, initializer));

            match self.curr() {
                Comma => {
                    self.advance()?;
                },
                In => {
                    self.advance()?;
                    break;
                }
                _ => {
                    return Err("Expected comma or 'in' keyword in variable declaration.".to_owned())
                }
            }
        }

        // parse body
        let body = self.parse_expr()?;

        Ok(Expr::VarIn {
            variables: variables,
            body: Box::new(body)
        })
    }

    /// Parses a primary expression (an identifier, a number or a parenthesized expression).
    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.curr() {
            Ident(_) => self.parse_id_expr(),
            Str(e) => { self.advance()?; Ok(Expr::Str(e)) },
            Number(_) => self.parse_nb_expr(),
            LParen => self.parse_paren_expr(),
            If => self.parse_conditional_expr(),
            For => self.parse_for_expr(),
            Var => self.parse_var_expr(),
            _ => Err("Unknown expression.".to_owned())
        }
    }

    /// Parses a top-level expression and makes an anonymous function out of it,
    /// for easier compilation.
    fn parse_toplevel_expr(&mut self) -> Result<Function, String> {
        match self.parse_expr() {
            Ok(expr) => {
                Ok(Function {
                    prototype: Prototype {
                        name: ANONYMOUS_FUNCTION_NAME.to_string(),
                        args: vec![],
                        is_op: false,
                        prec: 0

                    },
                    body: Some(expr),
                    is_anon: true,
                    cache: Cache::None,
                })
            },

            Err(err) => Err(err)
        }
    }
}
