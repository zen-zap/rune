#![allow(unused)]
use nix::{
    unistd::{pipe, fork, ForkResult, dup2, close, setpgid, write},
    sys::{
        wait::{waitpid, WaitPidFlag},
    },
};
use rune::UserCommand;
use rune::dispatcher;
use rune::parser;
use rune::read_line_from_fd;
use std::os::unix::process::CommandExt;
use std::os::unix::io::{BorrowedFd, RawFd, AsRawFd, FromRawFd, IntoRawFd, OwnedFd};
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn main() {
    let search_paths = dispatcher::load_paths();

    loop {
        let std_out_fd = unsafe { BorrowedFd::borrow_raw(1) };
        let _ = write(std_out_fd, b"RuneShell $ ").unwrap();

        let input = match read_line_from_fd(0) {
            Some(line) => line.trim().to_owned(),
            None => continue,
        };

        if input.is_empty() {
            continue;
        }

        let segments = input.split('|').map(str::trim).collect::<Vec<&str>>();
        let is_pipe = segments.len() > 1;

        if is_pipe {
            run_pipeline(segments, &search_paths);
        } else {
            run(input, &search_paths);
        }
    }
}

pub fn run(input: String, search_paths: &[PathBuf]) {
    let user_cmd: UserCommand = parser::parse(input.as_str()).expect("Failed to parse!");
    let cmd = user_cmd.cmd.clone();
    let args = user_cmd.args.clone();

    if dispatcher::builtin_check(&cmd) {
        dispatcher::builtin_process(cmd.as_str(), &user_cmd.args, 0, 1);
    } else {
        let exec_path = dispatcher::find_command(&cmd, search_paths);
        if exec_path.is_none() {
            println!("rune command not found: {cmd}");
            return;
        }
        let exe = exec_path.unwrap();

        match unsafe { fork() } {
            Ok(ForkResult::Child) => {
                let stdin  = unsafe { Stdio::from_raw_fd(0) };
                let stdout = unsafe { Stdio::from_raw_fd(1) };

                let mut cmd_proc = Command::new(&exe);
                cmd_proc.args(args.iter().map(|s| OsStr::new(s)))
                        .stdin(stdin)
                        .stdout(stdout)
                        .stderr(Stdio::inherit());

                let err = cmd_proc.exec();
                eprintln!("rune: failed to exec '{}': {}", cmd, err);
                std::process::exit(1);
            }
            Ok(ForkResult::Parent { child }) => {
                let _ = waitpid(child, None);
            }
            Err(err) => {
                eprintln!("fork failed: {}", err);
            }
        }
    }
}

pub fn run_pipeline(segments: Vec<&str>, search_paths: &[PathBuf]) {
    let mut commands = Vec::with_capacity(segments.len());
    for seg in &segments {
        let cmd = parser::parse(seg).expect("Failed to parse the command in given pipeline segment");
        commands.push(cmd);
    }

    let mut pids = Vec::with_capacity(commands.len());
    let mut input_fd: RawFd = 0;
    let mut pgid: Option<nix::unistd::Pid> = None;

    for (i, user_cmd) in commands.into_iter().enumerate() {
        let is_last = i == segments.len() - 1;

        let (read_fd, write_fd) = if !is_last {
            let (r, w) = pipe().unwrap();
            (Some(r), Some(w))
        } else {
            (None, None)
        };

        match unsafe { fork() } {
            Ok(ForkResult::Child) => {
                let pid = nix::unistd::getpid();

                if i == 0 {
                    setpgid(pid, pid).ok();
                } else if let Some(parent_pgid) = pgid {
                    setpgid(pid, parent_pgid).ok();
                }

                if input_fd != 0 {
                    dup2(input_fd, 0).unwrap();
                    close(input_fd).ok();
                }

                if let Some(w) = write_fd {
                    let raw_w = w.as_raw_fd();
                    dup2(raw_w, 1).unwrap();
                    let _ = w.into_raw_fd();
                }

                if let Some(r) = read_fd {
                    let _ = r.into_raw_fd();
                }

                if dispatcher::builtin_check(&user_cmd.cmd) {
                    dispatcher::builtin_process(&user_cmd.cmd, &user_cmd.args, 0, 1);
                    std::process::exit(0);
                } else {
                    let cmd = &user_cmd.cmd;
                    let exec_path = dispatcher::find_command(cmd.as_str(), search_paths);
                    if exec_path.is_none() {
                        println!("rune command not found: {}", cmd.as_str());
                        return;
                    }
                    let exe = exec_path.unwrap();

                    let stdin = unsafe { std::process::Stdio::from_raw_fd(0) };
                    let stdout = unsafe { std::process::Stdio::from_raw_fd(1) };

                    let mut cmd_proc = Command::new(&exe);
                    cmd_proc
                        .args(user_cmd.args.iter().map(|s| OsStr::new(s)))
                        .stdin(stdin)
                        .stdout(stdout)
                        .stderr(std::process::Stdio::inherit());

                    let err = cmd_proc.exec();
                    eprintln!("rune: failed to exec '{}' : {}", &user_cmd.cmd, err);
                    std::process::exit(1);
                }
            }
            Ok(ForkResult::Parent { child }) => {
                if i == 0 {
                    setpgid(child, child).ok();
                    pgid = Some(child);
                } else if let Some(parent_pgid) = pgid {
                    setpgid(child, parent_pgid).ok();
                }

                if let Some(w) = write_fd {
                    let raw_w = w.into_raw_fd();
                    nix::unistd::close(raw_w).unwrap();
                }

                if input_fd != 0 {
                    nix::unistd::close(input_fd).ok();
                }

                input_fd = if let Some(r) = read_fd {
                    let raw_r = r.into_raw_fd();
                    raw_r
                } else {
                    0
                };

                pids.push(child);
            }
            Err(err) => {
                eprintln!("fork() failed: {:?}", err);
                std::process::exit(1);
            }
        }
    }

    for pid in pids {
        waitpid(pid, Some(WaitPidFlag::WUNTRACED)).ok();
    }
}
