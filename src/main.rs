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
		dispatcher::builtin_process(cmd.as_str(), &user_cmd.args, 0, 1);
	} else {
		dispatcher::process_external(cmd.as_str(), &user_cmd.args, 0, 1);
	}
}

pub fn run_pipeline(segments: Vec<&str>) {

}
