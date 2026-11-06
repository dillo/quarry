//! Running external tools and capturing what they print.

use std::{path::Path, process::Command};

use crate::error::XtaskError;

/// Runs `program` with `args` in `cwd` and returns its standard output.
///
/// # Errors
/// Returns an error when the program cannot start, exits unsuccessfully, or
/// prints output that is not UTF-8.
pub fn run_capture(program: &str, args: &[&str], cwd: &Path) -> Result<String, XtaskError> {
    let command = format!("{program} {}", args.join(" "));
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| XtaskError::Command {
            command: command.clone(),
            detail: error.to_string(),
        })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(XtaskError::Command { command, detail });
    }
    String::from_utf8(output.stdout).map_err(|error| XtaskError::Command {
        command,
        detail: error.to_string(),
    })
}
