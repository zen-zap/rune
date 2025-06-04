#![allow(unused)]

use nix::unistd::write;
use rune::UserCommand;
use rune::dispatcher;
use rune::parser;
use rune::read_line_from_fd;
use std::io::{self, Write};
use std::os::unix::io::{BorrowedFd, RawFd};

fn main() {
	loop {
		let std_out_fd = unsafe { BorrowedFd::borrow_raw(0) };
		let _ = write(std_out_fd, b"RuneShell $ ").unwrap();

		// so to read input .. we need some file descriptor that provides us with the input fd
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
            run_pipeline(segments);
		} else {
			run(input);
		}
	}
}


/// Normal run if not pipeline
pub fn run(input: String) {
	let user_cmd: UserCommand = parser::parse(input.as_str())
        .expect("Failed to parse!");

	let cmd = user_cmd.cmd;
	let b_check = dispatcher::builtin_check(&cmd);

	println!("Builtin-Check: {}", b_check);

	if b_check {
        println!("Normal Execution");
		dispatcher::builtin_process(cmd.as_str(), &user_cmd.args, 0, 1);
	} else {
        println!("Pipe Execution");
		dispatcher::process_external(cmd.as_str(), &user_cmd.args, 0, 1);
	}
}


use nix::{
    unistd::{pipe, fork, ForkResult, dup2, close, setpgid, tcsetpgrp, Pid},
    sys::{
        wait::{waitpid, WaitPidFlag, WaitStatus},
        signal,
    },
};
use std::os::fd::AsRawFd;

/// To run a pipeline input
pub fn run_pipeline(segments: Vec<&str>) {

    let mut commands = Vec::with_capacity(segments.len());

    for seg in segments {
        commands.push(parser::parse(seg).expect("Failed to parse the command in given pipeline segment"));
    }

    let mut pids = Vec::with_capacity(commands.len());
    let mut input_fd: i32 = 0; // need stdin as the start point
    let mut pgid: Option<Pid> = None;

    for (i, user_cmd) in commands.clone().into_iter().enumerate() {


        // if this is the last then we have to give fd=1 (stdout) to the process
        let is_last = i == commands.len()-1;

        // pipe() returns (OwnedFd, OwnedFd) in nix 0.29
        let (read_fd, write_fd) = if !is_last {
            let (r, w) = pipe().unwrap();
            (Some(r), Some(w))
        } else {
            (None, None)
        };

        match unsafe { fork() } {
            
            Ok(ForkResult::Child) => {
                
                // setting process group for job control implementation later on
                let pid = nix::unistd::getpid();

                if i == 0 {
                    // if first child process then add this under the same process id
                    setpgid(pid, pid).ok();
                } else if let Some(job_pgid) = pgid {
                    setpgid(pid, job_pgid).ok();
                }

                // wire up FDs
                if input_fd != 0 {
                    nix::unistd::dup2(input_fd, 0).unwrap(); // input_fd is i32
                    nix::unistd::close(input_fd).ok();
                }

                if let Some(ref wfd) = write_fd {
                    nix::unistd::dup2(wfd.as_raw_fd(), 1).unwrap(); // convert OwnedFd to i32
                    nix::unistd::close(wfd.as_raw_fd()).ok();
                }

                if let Some(ref rfd) = read_fd {
                    nix::unistd::close(rfd.as_raw_fd()).ok(); // convert OwnedFd to i32
                }

                // Exec or builtin
                if dispatcher::builtin_check(&user_cmd.cmd) {
                    dispatcher::builtin_process(&user_cmd.cmd, &user_cmd.args, 0, 1);
                    std::process::exit(0);
                } else {
                    dispatcher::process_external(&user_cmd.cmd, &user_cmd.args, 0, 1);
                    std::process::exit(0);
                }
            }

            Ok(ForkResult::Parent { child }) => {

                let child_pid = child;

                if i == 0 {
                    setpgid(child_pid, child_pid).ok();
                    pgid = Some(child_pid);
                } else if let Some(job_pgid) = pgid {
                    setpgid(child_pid, job_pgid).ok();
                }

                if let Some(ref wfd) = write_fd {
                    nix::unistd::close(wfd.as_raw_fd()).ok(); // convert OwnedFd to i32
                }

                if input_fd != 0 {
                    nix::unistd::close(input_fd).ok();
                }

                input_fd = match read_fd {
                    Some(rfd) => rfd.as_raw_fd(), // convert OwnedFd to i32
                    None => 0,
                };

                pids.push(child_pid);
            }

            Err(_) => {
                panic!("Fork Failed!");
            }
        }
    }    

    // Wait for all children in the job
    for pid in pids {
        waitpid(pid, Some(WaitPidFlag::WUNTRACED)).ok();
    }
}
