use std::io::{self, Write, Read};
use std::process::{Command, Stdio};
use std::error::Error;

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
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

        if let Err(_) = invoke_cmd(list, true) {
            eprintln!("Command failed: {}", input.trim());
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

fn invoke_cmd(list: List, from_outside: bool) -> Result<String, Box<dyn Error>> {
    let cmd;
    let prev_stdout;
    let mut is_first = false;
    let is_last = from_outside;
    let stdin_cfg;
    let stdout_cfg;

    match list {
        Cons(c, l) => {
            if *l == Nil {
                is_first = true;
            }
            prev_stdout = invoke_cmd(*l, false)?;
            cmd = c;
        },
        Nil => return Ok(String::new()),
    };

    if is_first {
        stdin_cfg = Stdio::inherit();
    } else {
        stdin_cfg = Stdio::piped();
    }

    if is_last {
        stdout_cfg = Stdio::inherit();
    } else {
        stdout_cfg = Stdio::piped();
    }

    let mut child = Command::new(cmd.command)
        .args(cmd.args)
        .stdin(stdin_cfg)
        .stdout(stdout_cfg)
        .spawn()?;

    if !is_first {
        child.stdin.as_ref().unwrap().write_all(prev_stdout.as_bytes())?;
    }

    let mut s = String::new();

    if !is_last {
        child.stdout.unwrap().read_to_string(&mut s)?;
    } else {
        child.wait()?;
    }

    Ok(s)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn command() {
        let list = parse("true\n").unwrap();
        assert!(invoke_cmd(list, true).is_ok());
    }

    #[test]
    fn command_with_arguments() {
        let list = parse("true -l -a --test\n").unwrap();
        assert!(invoke_cmd(list, true).is_ok());
    }

    #[test]
    fn command_not_found() {
        let list = parse("NOTFOUND\n").unwrap();
        assert!(invoke_cmd(list, true).is_err());
    }

    #[test]
    fn command_empty() {
        let list = parse("\n").unwrap();
        assert!(invoke_cmd(list, true).is_ok());
    }

    #[test]
    fn command_pipe_two_commands() {
        let list = parse("ls | true\n").unwrap();
        assert!(invoke_cmd(list, true).is_ok());
    }

    #[test]
    fn command_pipe_three_commands() {
        let list = parse("ls | ls | true\n").unwrap();
        assert!(invoke_cmd(list, true).is_ok());
    }

    #[test]
    fn command_pipe_nospace() {
        let list = parse("ls|true\n").unwrap();
        assert!(invoke_cmd(list, true).is_ok());
    }

    #[test]
    fn command_pipe_first_command_not_found() {
        let list = parse("NOTFOUND | ls\n").unwrap();
        assert!(invoke_cmd(list, true).is_err());
    }

    #[test]
    fn command_pipe_second_command_not_found() {
        let list = parse("ls | NOTFOUND\n").unwrap();
        assert!(invoke_cmd(list, true).is_err());
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

    #[test]
    fn command_cat() {
        // TODO
    }
}
