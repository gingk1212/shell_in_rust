use std::process::{self, Command, Stdio, Child};
use std::error::Error;
use std::os::unix::io::{IntoRawFd, FromRawFd};
use std::fs::File;

#[derive(Debug)]
pub struct Cmd {
    command: String,
    args: Vec<String>,
    child: Option<Child>,
    is_redirect: bool,
    redirect_path: Option<String>,
    builtin: bool,
}

impl Cmd {
    fn new() -> Cmd {
        Cmd {
            command: String::new(),
            args: Vec::new(),
            child: None,
            is_redirect: false,
            redirect_path: None,
            builtin: false,
        }
    }
}

#[derive(Debug)]
pub enum List {
    Cons(Cmd, Box<List>),
    Nil,
}

use List::{Cons, Nil};

impl List {
    fn new() -> List {
        Nil
    }
}

pub fn parse(input: &str) -> Result<List, &str> {
    let mut list = List::new();
    let cmd_line: Vec<&str> = input.trim().split("|").collect();
    let cmd_num = cmd_line.len();

    for l in cmd_line {
        let mut cmd = Cmd::new();

        let cmd_str = match parse_redirect(l, &mut cmd) {
            Ok(s) => s,
            Err(e) => return Err(e),
        };

        let mut l = cmd_str.trim().split_whitespace();

        match l.next() {
            Some(s) => {
                if s == "exit" {
                    cmd.builtin = true;
                }
                cmd.command = String::from(s);
            },
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

fn parse_redirect(l: &str, cmd: &mut Cmd) -> Result<String, &'static str> {
    let l_vec: Vec<_> = l.split(">").collect();

    if l_vec.len() == 2 {
        let mut redirect_path_and_other = l_vec[1].trim().split_whitespace();
        match redirect_path_and_other.next() {
            Some(s) => cmd.redirect_path = Some(String::from(s)),
            None => return Err("Syntax error near unexpected '>'"),
        }
        cmd.is_redirect = true;
        let other: Vec<_> = redirect_path_and_other.collect();
        let s = l_vec[0].to_string() + " " + &other.join(" ");
        Ok(s)
    } else if l_vec.len() > 2 {     // Multiple '>' is not supported yet.
        Err("Syntax error near unexpected '>'")
    } else {
        Ok(l.to_string())
    }
}

// Called from the last command.
pub fn invoke_cmd(list: &mut List, from_outside: bool) -> Result<Option<&mut Cmd>, Box<dyn Error>> {
    let cmd;
    let is_last = from_outside;
    let stdin_cfg;
    let stdout_cfg;

    match list {
        Cons(c, l) => {
            stdin_cfg = get_stdin(l)?;
            cmd = c;
        },
        Nil => return Ok(None),
    };

    if cmd.is_redirect {
        let f = File::create(cmd.redirect_path.as_ref().unwrap())?;
        stdout_cfg = Stdio::from(f);
    } else if is_last {
        stdout_cfg = Stdio::inherit();
    } else {
        stdout_cfg = Stdio::piped();
    }

    if cmd.builtin {
        exec_exit();
    } else {
        cmd.child = match Command::new(&cmd.command)
            .args(&cmd.args)
            .stdin(stdin_cfg)
            .stdout(stdout_cfg)
            .spawn()
        {
            Ok(c) => Some(c),
            Err(e) => return Err(Box::new(e)),
        };
    }

    Ok(Some(cmd))
}

fn get_stdin(l: &mut List) -> Result<Stdio, Box<dyn Error>> {
    match invoke_cmd(l, false) {
        Ok(Some(cmd)) => {
            if cmd.is_redirect {
                Ok(Stdio::null())
            } else if cmd.builtin {
                Ok(Stdio::null())
            } else {
                let child = cmd.child.as_mut().unwrap();
                let stdout = child.stdout.take().unwrap();
                Ok(unsafe { Stdio::from_raw_fd(stdout.into_raw_fd()) })
            }
        },
        Ok(None) => Ok(Stdio::inherit()),
        Err(e) => return Err(e),
    }
}

fn exec_exit() {
    process::exit(0);
}

pub fn wait_cmdline(list: &mut List) -> Result<(), Box<dyn Error>> {
    let mut list_now = list;

    loop {
        match list_now {
            Cons(c, l) => {
                c.child.as_mut().unwrap().wait()?;
                list_now = l;
            },
            Nil => break,
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn command() {
        let mut list = parse("true\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
        assert!(wait_cmdline(&mut list).is_ok());
    }

    #[test]
    fn command_with_arguments() {
        let mut list = parse("true -l -a --test\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
        assert!(wait_cmdline(&mut list).is_ok());
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
        assert!(wait_cmdline(&mut list).is_ok());
    }

    #[test]
    fn command_pipe_two_commands() {
        let mut list = parse("ls | true\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
        assert!(wait_cmdline(&mut list).is_ok());
    }

    #[test]
    fn command_pipe_three_commands() {
        let mut list = parse("ls | ls | true\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
        assert!(wait_cmdline(&mut list).is_ok());
    }

    #[test]
    fn command_pipe_nospace() {
        let mut list = parse("ls|true\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
        assert!(wait_cmdline(&mut list).is_ok());
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
        let mut list = parse("ps -ef | true\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
        assert!(wait_cmdline(&mut list).is_ok());
    }

    #[test]
    fn command_command_on_the_way_take_stdin() {
        let mut list = parse("ls | wc -l | true\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
        assert!(wait_cmdline(&mut list).is_ok());
    }

    #[test]
    fn command_pipe_buffer_full() {
        let mut list = parse("ps -ef | ps -ef | true\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
        assert!(wait_cmdline(&mut list).is_ok());
    }

    #[test]
    fn command_redirect() {
        let mut list = parse("ls -l > /dev/null\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
        assert!(wait_cmdline(&mut list).is_ok());
    }

    #[test]
    fn command_redirect_nospace() {
        let mut list = parse("ls -l>/dev/null\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
        assert!(wait_cmdline(&mut list).is_ok());
    }

    #[test]
    fn command_redirect_front() {
        let mut list = parse("> /dev/null ls -l\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
        assert!(wait_cmdline(&mut list).is_ok());
    }

    #[test]
    fn command_redirect_middle() {
        let mut list = parse("ls > /dev/null -l\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
        assert!(wait_cmdline(&mut list).is_ok());
    }

    #[test]
    fn command_redirect_with_pipe() {
        let mut list = parse("ls -l > /dev/null | true\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
        assert!(wait_cmdline(&mut list).is_ok());
    }

    #[test]
    fn command_multiple_redirect() {
        assert!(parse("ls -l > hoge.txt > fuga.txt\n").is_err());
    }

    #[test]
    fn command_redirect_nopath() {
        assert!(parse("ls -l > \n").is_err());
    }

    // FIXME: These cause the test to stop on the way.
    // #[test]
    // fn command_builtin_exit() {
    //     let mut list = parse("exit\n").unwrap();
    //     assert!(invoke_cmd(&mut list, true).is_ok());
    //     assert!(wait_cmdline(&mut list).is_ok());
    // }

    // #[test]
    // fn command_builtin_exit_with_pipe() {
    //     let mut list = parse("exit | true\n").unwrap();
    //     assert!(invoke_cmd(&mut list, true).is_ok());
    //     assert!(wait_cmdline(&mut list).is_ok());
    // }

    // #[test]
    // fn command_builtin_exit_with_redirect() {
    //     let mut list = parse("exit > /dev/null\n").unwrap();
    //     assert!(invoke_cmd(&mut list, true).is_ok());
    //     assert!(wait_cmdline(&mut list).is_ok());
    // }
}
