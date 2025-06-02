use crate::UserCommand;

/// Parses the command and forms the UserCommand
pub fn parse(input: &str) -> Option<UserCommand> {

    let mut input_array = input.split_whitespace();

    let cmd = input_array.next()?.to_string();
    let args = input_array.map(|arg| arg.to_string()).collect();
    
    Some(UserCommand {
        cmd,
        args,
    })
}
