pub fn builtin_check(cmd: &String) -> bool {
    let check = match cmd.as_str() {
        "cd" => {
            true
        }
        "exit" => {
            true
        }
        _ => {
            false
        }
    };

    check
}

use std::env;
pub fn builtin_process(cmd: &String, args: Option<String>) {
}
