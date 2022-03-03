use std::io::{self, Write};
use std::process::Command;

fn main() {

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut command = String::new();
        io::stdin()
            .read_line(&mut command)
            .expect("Failed to read line");
        invoke_cmd(&command);
    }
}

fn invoke_cmd(cmd: &str) {
    let mut cmd = cmd.trim().split_whitespace();
    let first_cmd = match cmd.next() {
        Some(i) => i,
        None => return,
    };
    Command::new(first_cmd)
        .args(cmd)
        .status()
        .expect("Command not found");
}
