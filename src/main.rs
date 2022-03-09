use std::io::{self, Write};
use std::process::Command;
use std::error::Error;

#[derive(Debug)]
struct Cmd {
    command: String,
    args: Vec<String>,
}

impl Cmd {
    fn new() -> Cmd {
        Cmd {
            command: String::new(),
            args: Vec::new(),
        }
    }
}

#[derive(Debug)]
enum List {
    Cons(Cmd, Box<List>),
    Nil,
}

use List::{Cons, Nil};

impl List {
    fn new() -> List {
        Nil
    }
}

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => break, // EOF
            Ok(_) => (),
            Err(error) => panic!("Failed to read line: {:?}", error),
        };

        let list = List::new();
        let list = tokenize(list, &input).expect("Failed to tokenize");

        if let Err(_) = invoke_cmd(list) {
            eprintln!("Command not found: {}", input.trim());
        }
    }
}

fn tokenize(list: List, input: &str) -> Result<List, Box<dyn Error>> {
    let mut input = input.trim().split_whitespace();
    let mut cmd = Cmd::new();

    match input.next() {
        Some(s) => cmd.command = String::from(s),
        None => return Ok(list),
    }

    for arg in input {
        cmd.args.push(String::from(arg));
    }

    let list = Cons(cmd, Box::new(list));

    Ok(list)
}

fn invoke_cmd(list: List) -> Result<(), Box<dyn Error>> {
    let cmd;

    match list {
        Cons(c, _) => cmd = c,
        Nil => return Ok(()),
    };

    let mut child = Command::new(cmd.command)
        .args(cmd.args)
        .spawn()?;

    child.wait()?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn command() {
        let list = List::new();
        let list = tokenize(list, "true").unwrap();
        assert!(invoke_cmd(list).is_ok());
    }

    #[test]
    fn command_with_arguments() {
        let list = List::new();
        let list = tokenize(list, "true -l -a --test").unwrap();
        assert!(invoke_cmd(list).is_ok());
    }

    #[test]
    fn command_not_found() {
        let list = List::new();
        let list = tokenize(list, "NOTFOUND").unwrap();
        assert!(invoke_cmd(list).is_err());
    }

    #[test]
    fn command_empty() {
        let list = List::new();
        let list = tokenize(list, "").unwrap();
        assert!(invoke_cmd(list).is_ok());
    }
}
