use std::collections::HashSet;
use once_cell::sync::Lazy;
use std::env;
use std::path::{Path, PathBuf};
use std::os::unix::io::{RawFd, FromRawFd, BorrowedFd};
use nix::unistd::write;
use std::process::{Command, Stdio};
use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;

/// The set of supported shell built-in commands.
static BUILTINS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from(["cd", "pwd", "exit", "echo"])
});

/// Checks if the given command is a built-in shell command.
///
/// # Arguments
/// * `cmd` - The command name to check.
///
/// # Returns
/// * `true` if `cmd` is a built-in, `false` otherwise.
pub fn builtin_check(cmd: &str) -> bool {
    BUILTINS.contains(cmd)
}

/// Processes a built-in command, executing it with the given arguments and file descriptors.
///
/// # Arguments
/// * `cmd` - The built-in command to execute
/// * `args` - Arguments to the command
/// * `input_fd` - File descriptor to use for stdin
/// * `output_fd` - File descriptor to use for stdout
pub fn builtin_process(cmd: &str, args: &[String], input_fd: RawFd, output_fd: RawFd) {
    let _in_fd = unsafe { BorrowedFd::borrow_raw(input_fd) };
    let out_fd = unsafe { BorrowedFd::borrow_raw(output_fd) };
    match cmd {
        "exit" => {
            let code = args.get(0)
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);
            std::process::exit(code);
        }
        "cd" => {
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
                write(out_fd, err_msg.as_bytes()).ok();
            }
        }
        "pwd" => {
            if let Ok(curr_dir_path) = env::current_dir() {
                let output = format!("{}\n", curr_dir_path.display());
                write(out_fd, output.as_bytes()).ok();
            }
        }
        "echo" => {
            builtin_echo(args);
        }
        _ => {}
    };
}

/// Executes an external command, wiring up its stdin and stdout to the given file descriptors.
///
/// # Arguments
/// * `cmd` - command to run (e.g., "ls")
/// * `args` - arguments to pass (without the command itself)
/// * `input_fd` - file descriptor to use for stdin
/// * `output_fd` - file descriptor to use for stdout
pub fn process_external(cmd: &str, args: &[String], input_fd: RawFd, output_fd: RawFd) {
    let stdin = unsafe { Stdio::from_raw_fd(input_fd) };
    let stdout = unsafe { Stdio::from_raw_fd(output_fd) };
    let mut command = Command::new(cmd);
    command.args(args.iter().map(|s| OsStr::new(s)));
    command.stdin(stdin);
    command.stdout(stdout);
    command.stderr(Stdio::inherit());
    let error = command.exec();
    eprintln!("rune: failed to execute {}: {}", cmd, error);
    std::process::exit(1);
}

/// Loads the search paths from rune.conf at shell startup.
///
/// # Returns
/// * A vector of PathBuf representing search directories for executables.
pub fn load_paths() -> Vec<PathBuf> {
    let conf_path = "../rune.conf";
    std::fs::read_to_string(conf_path)
        .expect("Failed to read configuration file")
        .lines()
        .map(|s| PathBuf::from(s.trim()))
        .collect()
}

/// Implements the echo built-in command with basic -n flag support.
///
/// # Arguments
/// * `args` - Arguments to echo.
pub fn builtin_echo(args: &[String]) {
    let mut iter = args.iter();
    let mut no_newline = false;
    while let Some(arg) = iter.next() {
        if arg == "-n" {
            no_newline = true;
        } else {
            print!("{}", arg);
            break;
        }
    }
    for arg in iter {
        print!(" {}", arg);
    }
    if !no_newline {
        println!();
    } else {
        std::io::Write::flush(&mut std::io::stdout()).ok();
    }
}

/// Checks if a file at `path` exists and is executable.
///
/// # Arguments
/// * `path` - Path to check.
///
/// # Returns
/// * `true` if the file exists and is executable, `false` otherwise.
pub fn is_executable(path: &Path) -> bool {
    if let Ok(metadata) = fs::metadata(path) {
        let perms = metadata.permissions();
        metadata.is_file() && (perms.mode() & 0o111 != 0)
    } else {
        false
    }
}

/// Searches for a command in the given search paths.
///
/// If `cmd` contains a '/', treats it as a path and checks directly.
/// Otherwise, iterates through `search_paths` and returns the first match that is executable.
///
/// # Arguments
/// * `cmd` - Command to search for.
/// * `search_paths` - Slice of PathBufs to search.
///
/// # Returns
/// * Some(PathBuf) to the executable, or None if not found.
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
