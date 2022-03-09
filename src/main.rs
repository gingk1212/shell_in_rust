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

        let list = match parse(&input) {
            Ok(l) => l,
            Err(s) => {
                eprintln!("{}", s);
                continue;
            },
        };

        if let Err(_) = invoke_cmd(list) {
            eprintln!("Command not found: {}", input.trim());
        }
    }
}

fn parse(input: &str) -> Result<List, &str> {
    let mut list = List::new();
    let cmd_line: Vec<&str> = input.trim().split("|").collect();
    let cmd_num = cmd_line.len();

    for l in cmd_line {
        let mut l = l.trim().split_whitespace();
        let mut cmd = Cmd::new();

        match l.next() {
            Some(s) => cmd.command = String::from(s),
            None => {   // empty or whitespace command
                if cmd_num > 1 {
                    return Err("Syntax error near unexpected '|'");
                } else {
                    break;
                }
            },
        }

        for arg in l {
            cmd.args.push(String::from(arg));
        }

        list = Cons(cmd, Box::new(list));
    }

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
        let list = parse("true\n").unwrap();
        assert!(invoke_cmd(list).is_ok());
    }

    #[test]
    fn command_with_arguments() {
        let list = parse("true -l -a --test\n").unwrap();
        assert!(invoke_cmd(list).is_ok());
    }

    #[test]
    fn command_not_found() {
        let list = parse("NOTFOUND\n").unwrap();
        assert!(invoke_cmd(list).is_err());
    }

    #[test]
    fn command_empty() {
        let list = parse("\n").unwrap();
        assert!(invoke_cmd(list).is_ok());
    }

    #[test]
    fn command_pipe_two_commands() {
        let list = parse("ls | true\n").unwrap();
        assert!(invoke_cmd(list).is_ok());
    }

    #[test]
    fn command_pipe_three_commands() {
        let list = parse("ls | ls | true\n").unwrap();
        assert!(invoke_cmd(list).is_ok());
    }

    #[test]
    fn command_pipe_nospace() {
        let list = parse("ls|true\n").unwrap();
        assert!(invoke_cmd(list).is_ok());
    }

    #[test]
    fn command_pipe_first_command_not_found() {
        let list = parse("NOTFOUND | ls\n").unwrap();
        assert!(invoke_cmd(list).is_err());
    }

    #[test]
    fn command_pipe_second_command_not_found() {
        let list = parse("ls | NOTFOUND\n").unwrap();
        assert!(invoke_cmd(list).is_err());
    }

    #[test]
    fn command_pipe_first_command_does_not_exist() {
        assert!(parse("| ls\n").is_err());
    }

    #[test]
    fn command_pipe_second_command_does_not_exist() {
        assert!(parse("ls | \n").is_err());
    }

    #[test]
    fn command_pipe_only() {
        assert!(parse("|\n").is_err());
    }
}
