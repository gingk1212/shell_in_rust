use nix::{
    sys::wait::waitpid,
    unistd::{close, dup2, fork, pipe, ForkResult, Pid},
};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::os::unix::io::{FromRawFd, IntoRawFd, RawFd};
use std::path::Path;
use std::process::{self, Child, Command, Stdio};

#[derive(Debug)]
pub struct Cmd {
    command: String,
    args: Vec<String>,
    child: Option<Child>,
    redirect: bool,
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
            redirect: false,
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
                }
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
                if s == "exit" || s == "pwd" || s == "cd" {
                    cmd.builtin = true;
                }
                cmd.command = String::from(s);
            }
            None => {
                // empty or whitespace command
                if cmd_num > 1 {
                    return Err("Syntax error near unexpected '|'");
                } else {
                    break;
                }
            }
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
        cmd.redirect = true;
        let other: Vec<_> = redirect_path_and_other.collect();
        let s = l_vec[0].to_string() + " " + &other.join(" ");
        Ok(s)
    } else if l_vec.len() > 2 {
        // Multiple '>' is not supported yet.
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
        }
        Nil => return Ok(None),
    }

    if cmd.builtin {
        if single_command {
            if cmd.command == "exit" {
                exec_exit(cmd.args.len() as i32);
            } else if cmd.command == "pwd" {
                exec_pwd(
                    cmd.args.len() as i32,
                    &cmd.args,
                    cmd.redirect,
                    &cmd.redirect_path,
                )?;
            } else if cmd.command == "cd" {
                exec_cd(cmd.args.len() as i32, &cmd.args)?;
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
                if cmd.redirect {
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
            if c.redirect {
                Ok(Stdio::null())
            } else {
                Ok(unsafe { Stdio::from_raw_fd(c.fd0) })
            }
        }
        None => Ok(Stdio::inherit()),
    }
}

fn get_stdout(cmd: &mut Cmd, is_last: bool) -> Result<Stdio, Box<dyn Error>> {
    if cmd.redirect {
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

    if let Some(p) = prev_cmd {
        prev_fd0 = Some(p.fd0);
    }

    match unsafe { fork() } {
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
        }
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
            if cmd.redirect {
                let f = File::create(cmd.redirect_path.as_ref().unwrap())?;
                let fd = f.into_raw_fd();
                close(1)?;
                dup2(fd, 1)?;
                close(fd)?;

                // unnecessary
                close(fd1)?;
            } else if is_last {
                // unnecessary
                close(fd1)?;
            } else {
                close(1)?;
                dup2(fd1, 1)?;
                close(fd1)?;
            }

            if cmd.command == "exit" {
                exec_exit(cmd.args.len() as i32);
            } else if cmd.command == "pwd" {
                exec_pwd(
                    cmd.args.len() as i32,
                    &cmd.args,
                    cmd.redirect,
                    &cmd.redirect_path,
                )?;
            } else if cmd.command == "cd" {
                exec_cd(cmd.args.len() as i32, &cmd.args)?;
            }

            process::exit(0);
        }
        Err(e) => return Err(Box::new(e)),
    }

    Ok(())
}

fn exec_exit(argc: i32) {
    if argc != 0 {
        eprintln!("exit: wrong argument");
    } else {
        process::exit(0);
    }
}

fn exec_pwd(
    argc: i32,
    _args: &[String],
    redirect: bool,
    redirect_path: &Option<String>,
) -> Result<(), Box<dyn Error>> {
    if argc != 0 {
        eprintln!("pwd: wrong argument");
    } else {
        match env::current_dir() {
            Ok(s) => {
                if redirect {
                    let mut f = File::create(redirect_path.as_ref().unwrap())?;
                    f.write_all(s.to_str().unwrap().as_bytes())?;
                    f.write_all("\n".as_bytes())?;
                } else {
                    println!("{}", s.display());
                }
            }
            Err(_) => eprintln!("pwd: cannot get working directory"),
        }
    }

    Ok(())
}

fn exec_cd(argc: i32, args: &[String]) -> Result<(), Box<dyn Error>> {
    if argc != 1 {
        eprintln!("cd: wrong argument");
    } else {
        let dir = Path::new(&args[0]);
        env::set_current_dir(&dir)?;
    }

    Ok(())
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
            }
            Nil => break,
        }
    }

    Ok(())
}
