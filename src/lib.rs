use std::process::{self, Command, Stdio, Child};
use std::error::Error;
use std::os::unix::io::{IntoRawFd, FromRawFd, RawFd};
use std::fs::File;
use std::env;
use nix::{sys::wait::waitpid, unistd::{fork, ForkResult, Pid, pipe, close, dup2}};

#[derive(Debug)]
pub struct Cmd {
    command: String,
    args: Vec<String>,
    child: Option<Child>,
    is_redirect: bool,
    redirect_path: Option<String>,
    builtin: bool,
    pid: Option<Pid>,
    fd0: RawFd,
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
            pid: None,
            fd0: -1,
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

    fn get_cmd_num(&self) -> i32 {
        let mut list_now = self;
        let mut num = 0;

        loop {
            match list_now {
                Cons(_, l) => {
                    num += 1;
                    list_now = l;
                },
                Nil => break,
            }
        }

        num
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
                if s == "exit" || s == "pwd" {
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
pub fn invoke_cmd(list: &mut List, from_outside: bool) -> Result<Option<&Cmd>, Box<dyn Error>> {
    let cmd;
    let prev_cmd;
    let is_last = from_outside;
    let single_command = is_last && list.get_cmd_num() == 1;

    match list {
        Cons(c, l) => {
            prev_cmd = invoke_cmd(l, false)?;
            cmd = c;
        },
        Nil => return Ok(None),
    }

    if cmd.builtin {
        if single_command {
            if cmd.command == "exit" {
                exec_exit();
            } else if cmd.command == "pwd" {
                exec_pwd(cmd.args.len() as i32, &cmd.args);
            }
        } else {
            fork_exec(cmd, prev_cmd, is_last)?;
        }
    } else {
        let stdin_cfg = get_stdin(prev_cmd)?;
        let stdout_cfg = get_stdout(cmd, is_last)?;

        match Command::new(&cmd.command)
            .args(&cmd.args)
            .stdin(stdin_cfg)
            .stdout(stdout_cfg)
            .spawn()
        {
            Ok(mut c) => {
                if cmd.is_redirect {
                    let f = File::create("/dev/null")?;
                    cmd.fd0 = f.into_raw_fd();
                } else if !is_last {
                    let stdout = c.stdout.take().unwrap();
                    cmd.fd0 = stdout.into_raw_fd();
                }
                cmd.child = Some(c);
            }
            Err(e) => return Err(Box::new(e)),
        };
    }

    Ok(Some(cmd))
}

fn get_stdin(prev_cmd: Option<&Cmd>) -> Result<Stdio, Box<dyn Error>> {
    match prev_cmd {
        Some(c) => {
            if c.is_redirect {
                Ok(Stdio::null())
            } else {
                Ok(unsafe { Stdio::from_raw_fd(c.fd0) })
            }
        },
        None => Ok(Stdio::inherit())
    }
}

fn get_stdout(cmd: &mut Cmd, is_last: bool) -> Result<Stdio, Box<dyn Error>> {
    if cmd.is_redirect {
        let f = File::create(cmd.redirect_path.as_ref().unwrap())?;
        Ok(Stdio::from(f))
    } else if is_last {
        Ok(Stdio::inherit())
    } else {
        Ok(Stdio::piped())
    }
}

fn fork_exec(cmd: &mut Cmd, prev_cmd: Option<&Cmd>, is_last: bool) -> Result<(), Box<dyn Error>> {
    let (fd0, fd1) = pipe()?;
    let mut prev_fd0 = None;

    if let Some(_) = prev_cmd {
        prev_fd0 = Some(prev_cmd.unwrap().fd0);
    }

    match unsafe{fork()} {
        Ok(ForkResult::Parent { child }) => {
            cmd.pid = Some(child);
            close(fd1)?;

            if let Some(f) = prev_fd0 {
                close(f)?;
            }

            if is_last {
                close(fd0)?;
            } else {
                cmd.fd0 = fd0;
            }
        },
        Ok(ForkResult::Child) => {
            // unnecesssary
            close(fd0)?;

            // stdin
            if let Some(f) = prev_fd0 {
                close(0)?;
                dup2(f, 0)?;
                close(f)?;
            }

            // stdout
            if is_last {
                close(fd1)?;
            } else {
                close(1)?;
                dup2(fd1, 1)?;
                close(fd1)?;
            }

            if cmd.command == "exit" {
                exec_exit();
            } else if cmd.command == "pwd" {
                exec_pwd(cmd.args.len() as i32, &cmd.args);
            }

            process::exit(0);
        },
        Err(e) => return Err(Box::new(e)),
    }

    Ok(())
}

fn exec_exit() {
    process::exit(0);
}

fn exec_pwd(argc: i32, _args: &[String]) {
    if argc != 0 {
        eprintln!("pwd: wrong argument");
    } else {
        match env::current_dir() {
            Ok(s) => println!("{}", s.display()),
            Err(_) => eprintln!("pwd: cannot get working directory"),
        }
    }
}

pub fn wait_cmdline(list: &mut List) -> Result<(), Box<dyn Error>> {
    let mut list_now = list;

    loop {
        match list_now {
            Cons(c, l) => {
                if let Some(p) = c.pid {
                    waitpid(p, None)?;
                } else if let Some(_) = c.child {
                    c.child.as_mut().unwrap().wait()?;
                }
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

    // FIXME: This causes the test to stop on the way.
    // #[test]
    // fn command_builtin_exit() {
    //     let mut list = parse("exit\n").unwrap();
    //     assert!(invoke_cmd(&mut list, true).is_ok());
    //     assert!(wait_cmdline(&mut list).is_ok());
    // }

    #[test]
    fn command_builtin_exit_with_pipe() {
        let mut list = parse("exit | true\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
        assert!(wait_cmdline(&mut list).is_ok());
    }

    #[test]
    fn command_builtin_exit_with_pipe2() {
        let mut list = parse("true | exit\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
        assert!(wait_cmdline(&mut list).is_ok());
    }

    #[test]
    fn command_builtin_exit_with_pipe_and_redirect() {
        let mut list = parse("exit | ls -l > /dev/null\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
        assert!(wait_cmdline(&mut list).is_ok());
    }

    #[test]
    fn command_builtin_exit_with_pipe_and_redirect2() {
        let mut list = parse("ls -l > /dev/null | exit\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
        assert!(wait_cmdline(&mut list).is_ok());
    }

    #[test]
    fn command_builtin_pwd_with_redirect() {
        let mut list = parse("pwd > /dev/null\n").unwrap();
        assert!(invoke_cmd(&mut list, true).is_ok());
        assert!(wait_cmdline(&mut list).is_ok());
    }
}
