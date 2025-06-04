
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
use std::io::{self, Write};
use std::os::unix::process::CommandExt;
use std::fs;
use std::os::unix::io::{BorrowedFd, RawFd, AsRawFd, FromRawFd, IntoRawFd, OwnedFd};
use std::ffi::OsStr;
use std::process::{Command, Stdio};

fn main() {
    // Debug: print our PID so we can inspect /proc/<pid>/fd or lsof
    eprintln!("Debug: RuneShell PID = {}", std::process::id());

    loop {
        // Print prompt to stdout (fd 1)
        let std_out_fd = unsafe { BorrowedFd::borrow_raw(1) };
        let _ = write(std_out_fd, b"RuneShell $ <READY>\n").unwrap();
        eprintln!("Debug: Prompt written to fd 1");

        // so to read input .. we need some file descriptor that provides us with the input fd
        let input = match read_line_from_fd(0) {
            Some(line) => {
                eprintln!("Debug: Read line from stdin: {:?}", line);
                line.trim().to_owned()
            }
            None => {
                eprintln!("Debug: read_line_from_fd returned None, continuing");
                continue;
            }
        };

        if input.is_empty() {
            eprintln!("Debug: Input was empty, continuing");
            continue;
        }

        let segments = input.split('|').map(str::trim).collect::<Vec<&str>>();
        let is_pipe = segments.len() > 1;

        if is_pipe {
            println!("Pipe Execution");
            eprintln!("Debug: Detected pipeline with {} segments", segments.len());
            run_pipeline(segments);
        } else {
            println!("Normal Execution");
            eprintln!("Debug: Single command execution: {}", input);
            run(input);
        }
    }
}

/// Normal run if not pipeline
pub fn run(input: String) {
    eprintln!("Debug: Entering run() with input {:?}", input);

    let user_cmd: UserCommand = parser::parse(input.as_str())
        .expect("Failed to parse!");
    eprintln!("Debug: Parsed UserCommand: cmd='{}', args={:?}", user_cmd.cmd, user_cmd.args);

    let cmd = user_cmd.cmd.clone();
    let args = user_cmd.args.clone();
    let is_builtin = dispatcher::builtin_check(&cmd);
    eprintln!("Debug: builtin_check('{}') returned {}", cmd, is_builtin);

    println!("Builtin-Check: {}", is_builtin);

    if is_builtin {
        println!("Continuing with Normal Execution");
        eprintln!("Debug: Invoking builtin_process for '{}'", cmd);
        dispatcher::builtin_process(cmd.as_str(), &user_cmd.args, 0, 1);
    } else {

        println!("Builtin-Check: false -> forking for external");

        match unsafe { fork() } {
            Ok(ForkResult::Child) => {
                // Child: set up I/O redirection to fds 0 and 1 (terminal)
                eprintln!("[Child] Executing external '{}' with args {:?}", cmd, args);

                // Convert raw fds 0 and 1 into Stdio handles
                let stdin  = unsafe { Stdio::from_raw_fd(0) };
                let stdout = unsafe { Stdio::from_raw_fd(1) };

                // Build the command to exec
                let mut cmd_proc = Command::new(&cmd);
                cmd_proc.args(args.iter().map(|s| OsStr::new(s)))
                        .stdin(stdin)
                        .stdout(stdout)
                        .stderr(Stdio::inherit());

                // Replace the child with the external program
                let err = cmd_proc.exec();
                eprintln!("rune: failed to exec '{}': {}", cmd, err);
                std::process::exit(1);
            }
            Ok(ForkResult::Parent { child }) => {
                // Parent: wait for the child to finish
                eprintln!("[Parent] Forked child PID = {}", child);
                let _ = waitpid(child, None);
                eprintln!("[Parent] Child {} exited, returning to prompt", child);
                // After waitpid returns, control goes back to main loop (prompt)
            }
            Err(err) => {
                eprintln!("fork failed: {}", err);
            }        
        }
        
    }
}


/// To run a pipeline input
pub fn run_pipeline(segments: Vec<&str>) {
    // Debug: show the list of pipeline segments
    eprintln!("Debug: Starting run_pipeline with segments: {:?}", segments);

    // Parse each segment into a UserCommand ahead of time
    let mut commands = Vec::with_capacity(segments.len());
    for seg in &segments {
        let cmd = parser::parse(seg)
            .expect("Failed to parse the command in given pipeline segment");
        eprintln!("Debug: Parsed segment '{}' → cmd='{}', args={:?}", seg, cmd.cmd, cmd.args);
        commands.push(cmd);
    }

    let mut pids = Vec::with_capacity(commands.len());
    let mut input_fd: RawFd = 0;       // initially stdin of the shell
    let mut pgid: Option<nix::unistd::Pid> = None;

    for (i, user_cmd) in commands.into_iter().enumerate() {
        let is_last = i == segments.len() - 1;
        eprintln!(
            "Debug: Stage {} (is_last = {}), current input_fd = {}",
            i, is_last, input_fd
        );

        // Create a pipe for all but the last stage
        let (read_fd, write_fd) = if !is_last {
            let (r, w) = pipe().unwrap();
            eprintln!(
                "Debug: Created pipe for stage {} → read_fd={}, write_fd={}",
                i,
                r.as_raw_fd(),
                w.as_raw_fd()
            );
            (Some(r), Some(w))
        } else {
            eprintln!("Debug: No pipe for last stage {}", i);
            (None, None)
        };

        match unsafe { fork() } {
            Ok(ForkResult::Child) => {
                // child process
                let pid = nix::unistd::getpid();
                eprintln!("[Child-{}:{}] Forked (is_last={})", i, pid, is_last);

                // Set up process group so pipeline stages share a pgid
                if i == 0 {
                    // first child becomes its own group leader
                    setpgid(pid, pid).ok();
                    eprintln!("[Child-{}:{}] setpgid({}, {}) (new pgid)", i, pid, pid, pid);
                } else if let Some(parent_pgid) = pgid {
                    // subsequent children join the same pgid
                    setpgid(pid, parent_pgid).ok();
                    eprintln!(
                        "[Child-{}:{}] setpgid({}, {})",
                        i, pid, pid, parent_pgid
                    );
                }

                // Redirect stdin if input_fd ≠ 0
                if input_fd != 0 {
                    eprintln!("[Child-{}:{}] dup2({}, 0) to become stdin", i, pid, input_fd);
                    dup2(input_fd, 0).unwrap();
                    close(input_fd).ok();
                    eprintln!("[Child-{}:{}] closed original input_fd {}", i, pid, input_fd);
                }

                // Redirect stdout if write_fd exists
                if let Some(w) = write_fd {
                    let raw_w = w.as_raw_fd();
                    eprintln!(
                        "[Child-{}:{}] dup2({}, 1) to become stdout",
                        i, pid, raw_w
                    );
                    dup2(raw_w, 1).unwrap();
                    // We passed the raw fd into dup2, so drop the OwnedFd
                    let _ = w.into_raw_fd();
                    eprintln!("[Child-{}:{}] dropped OwnedFd for write_fd {}", i, pid, raw_w);
                }

                // If there is a read_fd for this stage, close it in the child
                if let Some(r) = read_fd {
                    let raw_r = r.into_raw_fd();
                    eprintln!("[Child-{}:{}] dropped OwnedFd for read_fd {}", i, pid, raw_r);
                }

                // Now execute the command. Built‐in vs external:
                if dispatcher::builtin_check(&user_cmd.cmd) {
                    eprintln!(
                        "[Child-{}:{}] Running builtin '{}'",
                        i, pid, &user_cmd.cmd
                    );
                    dispatcher::builtin_process(
                        &user_cmd.cmd,
                        &user_cmd.args,
                        0, // child’s stdin is already fd 0
                        1, // child’s stdout is already fd 1
                    );
                    std::process::exit(0);
                } else {
                    eprintln!(
                        "[Child-{}:{}] Running external '{}', args={:?}",
                        i, pid, &user_cmd.cmd, &user_cmd.args
                    );

                    // Convert fds 0 and 1 into Stdio handles for exec
                    let stdin = unsafe { std::process::Stdio::from_raw_fd(0) };
                    let stdout = unsafe { std::process::Stdio::from_raw_fd(1) };

                    // Build and exec the command
                    let mut cmd_proc = Command::new(&user_cmd.cmd);
                    cmd_proc
                        .args(user_cmd.args.iter().map(|s| OsStr::new(s)))
                        .stdin(stdin)
                        .stdout(stdout)
                        .stderr(std::process::Stdio::inherit());

                    let err = cmd_proc.exec();
                    eprintln!(
                        "rune: failed to exec '{}': {}",
                        &user_cmd.cmd, err
                    );
                    std::process::exit(1);
                }
            }

            Ok(ForkResult::Parent { child }) => {
                // parent process
                eprintln!("[Parent] Forked child {} for stage {}", child, i);

                // On first stage, child becomes group leader
                if i == 0 {
                    setpgid(child, child).ok();
                    pgid = Some(child);
                    eprintln!("[Parent] setpgid({}, {}) (new pgid)", child, child);
                } else if let Some(parent_pgid) = pgid {
                    setpgid(child, parent_pgid).ok();
                    eprintln!("[Parent] setpgid({}, {})", child, parent_pgid);
                }

                // Now parent closes its copy of write_fd (if any)
                if let Some(w) = write_fd {
                    let raw_w = w.into_raw_fd();
                    eprintln!(
                        "[Parent] closing write_fd={} so child 0 can see EOF",
                        raw_w
                    );
                    nix::unistd::close(raw_w).unwrap();
                }

                // Parent closes the old input_fd (read end of previous pipe), if any
                if input_fd != 0 {
                    eprintln!("[Parent] closing old input_fd={}", input_fd);
                    nix::unistd::close(input_fd).ok();
                }

                // If this wasn’t the last stage, set up input_fd for next iteration
                input_fd = if let Some(r) = read_fd {
                    let raw_r = r.into_raw_fd();
                    eprintln!(
                        "[Parent] new input_fd for next stage = {}",
                        raw_r
                    );
                    raw_r
                } else {
                    eprintln!("[Parent] resetting input_fd to 0 (stdin)");
                    0
                };

                pids.push(child);
                eprintln!("[Parent] pids so far: {:?}", pids);
            }

            Err(err) => {
                eprintln!("[Parent] fork() failed: {:?}", err);
                std::process::exit(1);
            }
        }
    }

    // After forking all stages, parent waits for each child
    eprintln!("[Parent] Waiting for children: {:?}", pids);
    for pid in pids {
        eprintln!("[Parent] waitpid({})", pid);
        waitpid(pid, Some(WaitPidFlag::WUNTRACED)).ok();
        eprintln!("[Parent] Child {} has exited", pid);
    }
    eprintln!("[Parent] All stages done, returning to main loop");
}

