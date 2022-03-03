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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn command() {
        assert!(invoke_cmd("true").is_ok());
    }

    #[test]
    fn command_with_arguments() {
        assert!(invoke_cmd("true -l -a --test").is_ok());
    }

    #[test]
    fn command_not_found() {
        assert!(invoke_cmd("NOTFOUND").is_err());
    }
}
