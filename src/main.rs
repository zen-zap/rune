use std::io::{self, Write};
use rune::dispatcher;
use rune::parser;
use rune::UserCommand;

fn main() {
    loop {
        print!(" >>> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut  input).is_err() {
            eprintln!("Failed to read input");
            continue;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        //println!("READ: \n {:?}", input);
        
        let user_cmd: UserCommand = parser::parse(input).expect("Failed to parse!");
        let cmd = user_cmd.cmd;
        let b_check = dispatcher::builtin_check(&cmd);

        println!("Builtin-Check: {}", b_check);

    }

}
