use std::io::{self, Write};
use std::process::Command;

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut command = String::new();
        match io::stdin().read_line(&mut command) {
            Ok(0) => break, // EOF
            Ok(size) => size,
            Err(error) => panic!("Failed to read line: {:?}", error),
        };
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
