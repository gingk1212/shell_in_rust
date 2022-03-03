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
    Command::new(cmd)
        .spawn()
        .expect("Command not found");
}
