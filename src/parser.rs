use crate::UserCommand;

/// Parses a shell command line into a `UserCommand` struct (command and arguments).
///
/// # Arguments
/// * `input` - The raw command line input string.
///
/// # Returns
/// * `Some(UserCommand)` if a command was found, or `None` for empty/whitespace input.
///
/// # Example
/// ```
/// let cmd = parse("ls -l /tmp").unwrap();
/// assert_eq!(cmd.cmd, "ls");
/// assert_eq!(cmd.args, vec!["-l", "/tmp"]);
/// ```
pub fn parse(input: &str) -> Option<UserCommand> {
    let mut tokens = input.split_whitespace();
    let cmd = tokens.next()?.to_string();
    let args = tokens.map(|arg| arg.to_string()).collect();

    Some(UserCommand { cmd, args })
}
