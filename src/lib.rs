#![allow(unused)]

pub mod dispatcher;
pub mod parser;

pub use parser::parse;
pub use dispatcher::{
    builtin_check,
    builtin_process,
};

/// Holds the user input command
pub struct UserCommand {
    pub cmd: String,
    pub args: Vec<String>,
}


use nix::unistd::read;
use std::os::unix::io::{
    RawFd,
    BorrowedFd,
};

/// takes a file descriptor and reads the content from the file descriptor
///
/// Pass valid file descriptor numbers
pub fn read_line_from_fd(fd: i32) -> Option<String>
{
    // temp buffer to read the contents into
    let mut buf = [0u8; 1024];

    // will hold the actual bytes after checking and stuff
    let mut line = Vec::new();

    // gotta make sure the given i32 refers to a valid Fd
    //
    // Now if you accidentally drop this fd it may lead to errors .. 
    // So this marks the fd as borrowed so dropping it doesn't close the fd
    let borrowed = unsafe {
        BorrowedFd::borrow_raw(fd)
    };

    loop {

        let n = read(borrowed, &mut buf).expect("read from fd failed");

        if n == 0 {
            break;
            // EOF
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
