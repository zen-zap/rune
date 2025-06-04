#![allow(unused)]


use std::collections::HashSet;
use once_cell::sync::Lazy;

static BUILTINS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from(["cd", "pwd", "echo", "exit"])
});

/// Checks if the given command is a built-in shell tool.
/// Accepts any string slice (`&str`).
pub fn builtin_check(cmd: &str) -> bool {
    BUILTINS.contains(&cmd)
}

use std::env;
use std::path::{
    Path,
    PathBuf,
};
use std::os::unix::io::{
    RawFd,
    BorrowedFd,
};
use nix::unistd::{
    write,
    read,
};

/// function to process any built-in commands
///
/// Accepts the input_fd and output_fd as RawFd
///
/// Verify with builtin_check first
pub fn builtin_process(cmd: &str, args: &[String], input_fd: RawFd, output_fd: RawFd) {

    let in_fd = unsafe {
        BorrowedFd::borrow_raw(input_fd)
    };
    let out_fd = unsafe {
        BorrowedFd::borrow_raw(output_fd)
    };


    let _ = match cmd {

        "exit" => {
            let code = args.get(0)
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);
                
            std::process::exit(code);
        }
        "cd" => {

            let curr_dir_path = env::current_dir().unwrap().as_path();

            let dest: String = args.get(0)
                .cloned()
                .or_else(|| env::var("HOME").ok())
                .unwrap_or_else(|| String::from("/"));
            
            let dest_path = Path::new(&dest);
            let final_dir = if dest_path.is_absolute() {
                dest_path.to_path_buf()
            } else {
                env::current_dir().unwrap().join(dest_path)
            };

            if let Err(e) = env::set_current_dir(&final_dir) {
                let err_msg = format!("failed to change directories!\ncd {} : {}", dest_path.display(), e);

                write(out_fd, err_msg.as_bytes())
                    .expect("Failed to write into output fd");
            }
        }
        "pwd" => {

            let curr_dir_path = env::current_dir()
                .expect("Failed to get the current directory");

            let output = format!("{}\n", curr_dir_path.display());

            write(out_fd, output.as_bytes())
                .expect("Failed to write to output file descriptor");

            
        }
        _ => {
            unimplemented!()
        }
    };
}

/// function to execute external commands
///
/// Accepts an input_fd and output_fd as RawFd
pub fn process_external(cmd: &str, args: &[String], input_fd: RawFd, output_fd: RawFd) {

    let in_fd = unsafe {
        BorrowedFd::borrow_raw(input_fd)
    };

    let out_fd = unsafe {
        BorrowedFd::borrow_raw(output_fd)
    };
}


/// Function to load the configuration files at shell start-up
pub fn load_paths() -> Vec<PathBuf> {

    let conf_path = "../rune.conf";

    let search_paths = std::fs::read_to_string(conf_path)
        .expect("Failed to read configuration file")
        .lines()
        .map(|s| PathBuf::from(s.trim()))
        .collect();

    search_paths
}
