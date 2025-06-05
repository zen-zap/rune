#![allow(unused)]


use std::collections::HashSet;
use once_cell::sync::Lazy;

static BUILTINS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from(["cd", "pwd", "exit", "echo"])
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
    FromRawFd,
    BorrowedFd,
};
use nix::unistd::{
    write,
    read,
};
use std::os::unix::process::CommandExt;

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

        "echo" => {
            builtin_echo(args);
        }
        _ => {
            unimplemented!()
        }
    };
}

use std::process::{Command, Stdio};
use std::ffi::OsStr;

/// Executes an external command, wiring up its stdin and stdout to the given file descriptors.
/// - `cmd`: the command to run (e.g., "ls")
/// - `args`: the arguments to pass (without the command itself)
/// - `input_fd`: the file descriptor to use for stdin
/// - `output_fd`: the file descriptor to use for stdout
pub fn process_external(cmd: &str, args: &[String], input_fd: RawFd, output_fd: RawFd) {

    let stdin = unsafe {
        Stdio::from_raw_fd(input_fd)
    };
    let stdout = unsafe {
        Stdio::from_raw_fd(output_fd)
    };

    let mut command = Command::new(cmd);

    command.args(args.iter().map(|s| OsStr::new(s)));
    // Set up redirected stdin/stdout
    command.stdin(stdin);
    command.stdout(stdout);
    // Set stderr to inherit from the shell, so errors are visible
    command.stderr(Stdio::inherit());

    // Actually exec the command, replacing the child process image
    // This will NOT return if successful
    let error = command.exec();
    // If exec fails, print error and exit
    eprintln!("rune: failed to execute {}: {}", cmd, error);
    std::process::exit(1);
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

pub fn builtin_echo(args: &[String]) {
    let mut iter = args.iter();
    let mut no_newline = false;

    // Check for -n flag(s)
    while let Some(arg) = iter.next() {
        if arg == "-n" {
            no_newline = true;
        } else {
            print!("{}", arg);
            break;
        }
    }

    // Print the rest of the args with spaces
    for arg in iter {
        print!(" {}", arg);
    }

    if !no_newline {
        println!();
    } else {
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
    }
}

use std::fs;
use std::os::unix::fs::PermissionsExt;

pub fn is_executable(path: &Path) -> bool {

    if let Ok(metadata) = fs::metadata(&path) {
        let perms = metadata.permissions();
        metadata.is_file() && (perms.mode() & 0o111 != 0)
    } else {
        false
    }
}

pub fn find_command(cmd: &str, search_paths: &[PathBuf]) -> Option<PathBuf> {

    if cmd.contains('/') {
        let path = PathBuf::from(cmd);
        if is_executable(&path) {
            return Some(path);
        } else {
            return None;
        }
    }

    for dir in search_paths {
        let candidate = dir.join(cmd);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }

    None
}
