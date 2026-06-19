use std::process::Output;

/// Abstraction over process execution, enabling testing without real I/O.
pub trait Runner {
    /// Run a command and return its output. Returns `None` if the command
    /// failed to execute (not if it exited with non-zero status).
    fn run_command(&self, program: &str, args: &[&str]) -> Option<Output>;

    /// Exit the process with the given code.
    fn exit(&self, code: i32) -> !;
}

/// Real process runner — executes actual commands and exits the process.
pub struct RealRunner;

impl Runner for RealRunner {
    fn run_command(&self, program: &str, args: &[&str]) -> Option<Output> {
        std::process::Command::new(program).args(args).output().ok()
    }

    fn exit(&self, code: i32) -> ! {
        std::process::exit(code)
    }
}
