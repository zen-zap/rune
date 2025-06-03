#![allow(unused)]

/// function to check if the given command is a built-in tool
pub fn builtin_check(cmd: &String) -> bool {
    let check = match cmd.as_str() {
        "cd" => {
            true
        }
        "exit" => {
            true
        }
        "pwd" => {
            true
        }
        "echo" => {
            true
        }
        _ => {
            false
        }
    };

    check
}


use std::env;
use std::path::{Path};

/// function to process any built-in commands
///
/// Verify with builtin_check first
pub fn builtin_process(cmd: &str, args: &[String]) {

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
                eprintln!("failed to change directories!\ncd {} : {}", dest_path.display(), e);
            }
        }
        "pwd" => {

            let curr_dir_path = env::current_dir()
                .expect("Failed to get the current directory");

            println!("{}", curr_dir_path.display());
        }
        _ => {
            unimplemented!()
        }
    };
}

/// Build to execute external commands
pub fn process_external(cmd: &str, args: &[String]) {
    let mut child = std::process::Command::new(cmd)
        .args(args)
        .spawn()
        .expect("Failed to spawn process for execution!");

    child.wait().expect("Failed to wait for the child");
}
