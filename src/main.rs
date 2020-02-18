extern crate rustyline;

use rustyline::error::ReadlineError;
use rustyline::Editor;

mod lex;
mod parse;

use lex::*;
use parse::*;
use std::collections::HashMap;

fn main() {
    // Build precedence map
    let mut prec = HashMap::with_capacity(6);

    prec.insert('=', 2);
    prec.insert('<', 10);
    prec.insert('+', 20);
    prec.insert('-', 20);
    prec.insert('*', 40);
    prec.insert('/', 40);


    // `()` can be used when no completer is required
    let mut rl = Editor::<()>::new();
    if rl.load_history(".reggae.history").is_err() {
        println!("No previous history.");
    }
    loop {
        let readline = rl.readline(">>->");
        match readline {
            Ok(line) => {
                rl.add_history_entry(&line);
                let res = Lexer::new(&(line.clone() + "\n")).collect::<Vec<Token>>();
                println!("-> Attempting to parse lexed input: \n{:?}\n", res);
                let res = Parser::new(line + "\n", &mut prec).parse();
                println!("-> Attempting to parse lexed input: \n{:?}\n", res);

            },
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break
            },
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break
            },
            Err(err) => {
                println!("Error: {:?}", err);
                break
            }
        }
    }
    rl.save_history(".reggae.history").unwrap();
}