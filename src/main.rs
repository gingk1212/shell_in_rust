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
    let output = Command::new(cmd)
        .output()
        .expect("Command not found");
    io::stdout().write_all(&output.stdout).unwrap();
    io::stdout().write_all(&output.stderr).unwrap();
}
