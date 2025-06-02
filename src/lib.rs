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
