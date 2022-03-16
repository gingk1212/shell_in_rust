use std::io::{self, Write};
use std::process::{Command, Stdio, Child};
use std::error::Error;
use std::os::unix::io::{IntoRawFd, FromRawFd};

#[derive(Debug)]
struct Cmd {
    command: String,
    args: Vec<String>,
    child: Option<Child>,
}

impl Cmd {
    fn new() -> Cmd {
        Cmd {
            command: String::new(),
            args: Vec::new(),
            child: None,
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

        let mut list = match parse(&input) {
            Ok(l) => l,
            Err(s) => {
                eprintln!("{}", s);
                continue;
            },
        };

        if let Err(e) = invoke_cmd(&mut list, true) {
            eprintln!("Command failed: {}", input.trim());
            if cfg!(debug_assertions) {
                eprintln!("{}", e);
            }
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

fn invoke_cmd(list: &mut List, from_outside: bool) -> Result<Option<&mut Child>, Box<dyn Error>> {
    let cmd;
    let mut prev_child = None;
    let mut is_first = false;
    let is_last = from_outside;
    let stdin_cfg;
    let stdout_cfg;

    match list {
        Cons(c, l) => {
            match invoke_cmd(l, false) {
                Ok(Some(child)) => prev_child = Some(child),
                Ok(None) => is_first = true,
                Err(e) => return Err(e),
            }
            cmd = c;
        },
        Nil => return Ok(None),
    };

    if is_first {
        stdin_cfg = Stdio::inherit();
    } else {
        let prev_stdout = prev_child.unwrap().stdout.take().unwrap();
        stdin_cfg = unsafe { Stdio::from_raw_fd(prev_stdout.into_raw_fd()) };
    }

    if is_last {
        stdout_cfg = Stdio::inherit();
    } else {
        stdout_cfg = Stdio::piped();
    }

    cmd.child = match Command::new(&cmd.command)
        .args(&cmd.args)
        .stdin(stdin_cfg)
        .stdout(stdout_cfg)
        .spawn()
    {
        Ok(c) => Some(c),
        Err(e) => return Err(Box::new(e)),
    };

    if is_last {
        cmd.child.as_mut().unwrap().wait()?;
    }

    Ok(cmd.child.as_mut())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn command() {
        let mut list = parse("true\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
    }

    #[test]
    fn command_with_arguments() {
        let mut list = parse("true -l -a --test\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
    }

    #[test]
    fn command_not_found() {
        let mut list = parse("NOTFOUND\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_err());
    }

    #[test]
    fn command_empty() {
        let mut list = parse("\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
    }

    #[test]
    fn command_pipe_two_commands() {
        let mut list = parse("ls | true\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
    }

    #[test]
    fn command_pipe_three_commands() {
        let mut list = parse("ls | ls | true\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
    }

    #[test]
    fn command_pipe_nospace() {
        let mut list = parse("ls|true\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
    }

    #[test]
    fn command_pipe_first_command_not_found() {
        let mut list = parse("NOTFOUND | ls\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_err());
    }

    #[test]
    fn command_pipe_second_command_not_found() {
        let mut list = parse("ls | NOTFOUND\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_err());
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

    #[test]
    fn command_second_command_does_not_take_stdin() {
        let mut list = parse("ss | true\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
    }

    #[test]
    fn command_command_on_the_way_take_stdin() {
        let mut list = parse("ls | wc -l | true\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
    }

    #[test]
    fn command_ss_ss() {
        let mut list = parse("ss | ss | true\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
    }
}
