use shell_in_rust;

#[test]
fn single_command() {
    let mut list = shell_in_rust::parse("true\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_ok());
    assert!(shell_in_rust::wait_cmdline(&mut list).is_ok());
}

#[test]
fn command_with_arguments() {
    let mut list = shell_in_rust::parse("true -l -a --test\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_ok());
    assert!(shell_in_rust::wait_cmdline(&mut list).is_ok());
}

#[test]
fn command_not_found() {
    let mut list = shell_in_rust::parse("NOTFOUND\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_err());
}

#[test]
fn empty() {
    let mut list = shell_in_rust::parse("\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_ok());
    assert!(shell_in_rust::wait_cmdline(&mut list).is_ok());
}

#[test]
fn pipe_two_commands() {
    let mut list = shell_in_rust::parse("ls | true\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_ok());
    assert!(shell_in_rust::wait_cmdline(&mut list).is_ok());
}

#[test]
fn pipe_three_commands() {
    let mut list = shell_in_rust::parse("ls | ls | true\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_ok());
    assert!(shell_in_rust::wait_cmdline(&mut list).is_ok());
}

#[test]
fn pipe_nospace() {
    let mut list = shell_in_rust::parse("ls|true\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_ok());
    assert!(shell_in_rust::wait_cmdline(&mut list).is_ok());
}

#[test]
fn pipe_first_command_not_found() {
    let mut list = shell_in_rust::parse("NOTFOUND | ls\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_err());
}

#[test]
fn pipe_second_command_not_found() {
    let mut list = shell_in_rust::parse("ls | NOTFOUND\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_err());
}

#[test]
fn pipe_first_command_does_not_exist() {
    assert!(shell_in_rust::parse("| ls\n").is_err());
}

#[test]
fn pipe_second_command_does_not_exist() {
    assert!(shell_in_rust::parse("ls | \n").is_err());
}

#[test]
fn pipe_only() {
    assert!(shell_in_rust::parse("|\n").is_err());
}

#[test]
fn cat() {
    // TODO: `cat` takes stdin.
}

#[test]
fn second_command_does_not_take_stdin() {
    let mut list = shell_in_rust::parse("ps -ef | true\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_ok());
    assert!(shell_in_rust::wait_cmdline(&mut list).is_ok());
}

#[test]
fn command_on_the_way_take_stdin() {
    let mut list = shell_in_rust::parse("ls | wc -l | true\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_ok());
    assert!(shell_in_rust::wait_cmdline(&mut list).is_ok());
}

#[test]
fn pipe_buffer_full() {
    let mut list = shell_in_rust::parse("ps -ef | ps -ef | true\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_ok());
    assert!(shell_in_rust::wait_cmdline(&mut list).is_ok());
}

#[test]
fn redirect() {
    let mut list = shell_in_rust::parse("ls -l > /dev/null\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_ok());
    assert!(shell_in_rust::wait_cmdline(&mut list).is_ok());
}

#[test]
fn redirect_nospace() {
    let mut list = shell_in_rust::parse("ls -l>/dev/null\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_ok());
    assert!(shell_in_rust::wait_cmdline(&mut list).is_ok());
}

#[test]
fn redirect_front() {
    let mut list = shell_in_rust::parse("> /dev/null ls -l\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_ok());
    assert!(shell_in_rust::wait_cmdline(&mut list).is_ok());
}

#[test]
fn redirect_middle() {
    let mut list = shell_in_rust::parse("ls > /dev/null -l\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_ok());
    assert!(shell_in_rust::wait_cmdline(&mut list).is_ok());
}

#[test]
fn redirect_with_pipe() {
    let mut list = shell_in_rust::parse("ls -l > /dev/null | true\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_ok());
    assert!(shell_in_rust::wait_cmdline(&mut list).is_ok());
}

#[test]
fn multiple_redirect() {
    assert!(shell_in_rust::parse("ls -l > hoge.txt > fuga.txt\n").is_err());
}

#[test]
fn redirect_nopath() {
    assert!(shell_in_rust::parse("ls -l > \n").is_err());
}

// FIXME: This causes the test to stop on the way.
// #[test]
// fn builtin_exit() {
//     let mut list = shell_in_rust::parse("exit\n").unwrap();
//     assert!(shell_in_rust::invoke_cmd(&mut list, true).is_ok());
//     assert!(shell_in_rust::wait_cmdline(&mut list).is_ok());
// }

#[test]
fn builtin_exit_with_pipe() {
    let mut list = shell_in_rust::parse("exit | true\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_ok());
    assert!(shell_in_rust::wait_cmdline(&mut list).is_ok());
}

#[test]
fn builtin_exit_with_pipe2() {
    let mut list = shell_in_rust::parse("true | exit\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_ok());
    assert!(shell_in_rust::wait_cmdline(&mut list).is_ok());
}

#[test]
fn builtin_exit_with_pipe_and_redirect() {
    let mut list = shell_in_rust::parse("exit | ls -l > /dev/null\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_ok());
    assert!(shell_in_rust::wait_cmdline(&mut list).is_ok());
}

#[test]
fn builtin_exit_with_pipe_and_redirect2() {
    let mut list = shell_in_rust::parse("ls -l > /dev/null | exit\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_ok());
    assert!(shell_in_rust::wait_cmdline(&mut list).is_ok());
}

#[test]
fn builtin_pwd_with_redirect() {
    let mut list = shell_in_rust::parse("pwd > /dev/null\n").unwrap();
    assert!(shell_in_rust::invoke_cmd(&mut list, true).is_ok());
    assert!(shell_in_rust::wait_cmdline(&mut list).is_ok());
}
