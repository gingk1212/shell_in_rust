use std::io::{self, Write};
use std::process::Command;

fn main() {
    let mut command = String::new();

    print!("$ ");
    io::stdout().flush().unwrap();
    io::stdin()
        .read_line(&mut command)
        .expect("Failed to read line");
    let command = command.trim();
    invoke_cmd(command);
}

fn invoke_cmd(cmd: &str) {
    let mut cmd = cmd.split_whitespace();
    let first_cmd = match cmd.next() {
        Some(i) => i,
        None => return,
    };
    Command::new(first_cmd)
        .args(cmd)
        .spawn()
        .expect("Command not found");
}
