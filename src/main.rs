use std::io::{self, Write};
use std::process::Command;
use std::error::Error;

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
        if let Err(_) = invoke_cmd(&command) {
            eprintln!("Command not found: {}", command.trim());
        }
    }
}

fn invoke_cmd(cmd: &str) -> Result<(), Box<dyn Error>> {
    let mut cmd = cmd.trim().split_whitespace();
    let first_cmd = match cmd.next() {
        Some(i) => i,
        None => return Ok(()),
    };

    Command::new(first_cmd)
        .args(cmd)
        .status()?;

    Ok(())
}
