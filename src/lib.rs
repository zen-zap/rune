pub mod dispatcher;
pub mod parser;

pub use parser::parse;
pub use dispatcher::{builtin_check, builtin_process};

/// Represents a parsed user command and its arguments.
#[derive(Clone, Debug)]
pub struct UserCommand {
    pub cmd: String,
    pub args: Vec<String>,
}

use nix::unistd::read;
use std::os::unix::io::RawFd;

/// Reads a single line from the given file descriptor (`fd`), returning it as a `String`.
///
/// Reads bytes until a newline (`\n`) or EOF is encountered. Returns `None` if no bytes are read.
///
/// # Arguments
/// * `fd` - The file descriptor to read from (e.g., 0 for stdin)
///
/// # Returns
/// * `Some(String)` containing the line (without the trailing newline), or `None` on EOF.
///
/// # Panics
/// Panics if the underlying `read()` syscall fails.
pub fn read_line_from_fd(fd: RawFd) -> Option<String> {
    let mut buf = [0u8; 1024];
    let mut line = Vec::new();

    loop {
        let n = read(fd, &mut buf).expect("read from fd failed");
        if n == 0 {
            break;
        }
        for &byte in &buf[..n] {
            if byte == b'\n' {
                return Some(String::from_utf8_lossy(&line).into_owned());
            }
            line.push(byte);
        }
    }

    if line.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(&line).into_owned())
    }
}
