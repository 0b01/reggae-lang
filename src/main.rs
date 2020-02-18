mod lex;
use lex::*;
use std::io::{self, Write};
use std::collections::HashMap;

fn main() {
    loop {
        println!();
        print!("?> ");
        io::stdout().flush().unwrap();

        // Read input from stdin
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Could not read from standard input.");

        if input.starts_with("exit") || input.starts_with("quit") {
            break;
        } else if input.chars().all(char::is_whitespace) {
            continue;
        }

        // Build precedence map
        let mut prec = HashMap::with_capacity(6);

        prec.insert('=', 2);
        prec.insert('<', 10);
        prec.insert('+', 20);
        prec.insert('-', 20);
        prec.insert('*', 40);
        prec.insert('/', 40);

        println!("-> Attempting to parse lexed input: \n{:?}\n", Lexer::new(input.as_str()).collect::<Vec<Token>>());

    }
}
