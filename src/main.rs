use std::io::{self, Write};
use std::process::Command;
use std::error::Error;

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => break, // EOF
            Ok(_) => (),
            Err(error) => panic!("Failed to read line: {:?}", error),
        };
        if let Err(_) = invoke_cmd(&input) {
            eprintln!("Command not found: {}", input.trim());
        }
    }
}

fn invoke_cmd(input: &str) -> Result<(), Box<dyn Error>> {
    let mut input = input.trim().split_whitespace();
    let first_cmd = match input.next() {
        Some(s) => s,
        None => return Ok(()),
    };

    let mut child = Command::new(first_cmd)
        .args(input)
        .spawn()?;

    child.wait()?;

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
