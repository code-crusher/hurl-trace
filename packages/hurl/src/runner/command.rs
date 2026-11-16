/*
 * Hurl (https://hurl.dev)
 * Copyright (C) 2026 Orange
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *          http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 */
use std::process::Command as ProcessCommand;

use hurl_core::ast::{Command, CommandName, SourceInfo};

use super::error::{RunnerError, RunnerErrorKind};
use super::template::eval_template;
use super::variable::VariableSet;

/// The result of a command run by Hurl, either before or after the HTTP request of an entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandResult {
    pub name: CommandName,
    pub ret_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// The results of the commands run by an entry, in their execution order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommandResults {
    results: Vec<CommandResult>,
}

impl CommandResults {
    /// Returns the result of the last command run with the `name`, if any.
    pub fn get(&self, name: &str) -> Option<&CommandResult> {
        self.results
            .iter()
            .rev()
            .find(|r| r.name.identifier() == name)
    }

    /// Adds a command `result`.
    pub fn add(&mut self, result: CommandResult) {
        self.results.push(result);
    }
}

/// Runs the `commands` of an entry that are declared with `name`, given a set of `variables`.
///
/// Commands are run with the system shell (`sh -c` on Unix, `cmd /C` on Windows), in the order
/// they are declared. A command exiting with a non-zero return code is not an error: the return
/// code is exposed to the asserts so users can check it.
pub fn eval_commands(
    commands: &[Command],
    name: CommandName,
    variables: &VariableSet,
) -> Result<Vec<CommandResult>, RunnerError> {
    let mut results = vec![];
    for command in commands.iter().filter(|c| c.name == name) {
        let value = eval_template(&command.command, variables)?;
        let result = run_command(name, &value, command.command.source_info)?;
        results.push(result);
    }
    Ok(results)
}

/// Runs `value` in a system shell and returns its result.
fn run_command(
    name: CommandName,
    value: &str,
    source_info: SourceInfo,
) -> Result<CommandResult, RunnerError> {
    let mut process = shell(value);
    let output = process.output().map_err(|e| {
        let kind = RunnerErrorKind::CommandExecution {
            command: value.to_string(),
            error: e.to_string(),
        };
        RunnerError::new(source_info, kind, false)
    })?;
    Ok(CommandResult {
        name,
        // A process killed by a signal has no exit code, we report -1.
        ret_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

/// Returns a [`ProcessCommand`] that runs `value` with the system shell.
fn shell(value: &str) -> ProcessCommand {
    #[cfg(windows)]
    {
        let mut process = ProcessCommand::new("cmd");
        process.arg("/C").arg(value);
        process
    }
    #[cfg(not(windows))]
    {
        let mut process = ProcessCommand::new("sh");
        process.arg("-c").arg(value);
        process
    }
}

#[cfg(test)]
mod tests {
    use hurl_core::reader::Pos;

    use super::*;

    #[test]
    fn test_eval_commands_with_no_command() {
        let variables = VariableSet::new();
        let results = eval_commands(&[], CommandName::PreRequest, &variables).unwrap();
        assert_eq!(results, vec![]);
    }

    #[test]
    fn test_run_command_stdout_stderr() {
        let source_info = SourceInfo::new(Pos::new(1, 1), Pos::new(1, 1));
        let result = run_command(
            CommandName::PreRequest,
            "echo out; echo err 1>&2",
            source_info,
        )
        .unwrap();
        assert_eq!(result.name, CommandName::PreRequest);
        assert_eq!(result.ret_code, 0);
        assert_eq!(result.stdout, "out\n");
        assert_eq!(result.stderr, "err\n");
    }

    #[test]
    fn test_run_command_ret_code() {
        let source_info = SourceInfo::new(Pos::new(1, 1), Pos::new(1, 1));
        let result = run_command(CommandName::PostRequest, "exit 3", source_info).unwrap();
        assert_eq!(result.name, CommandName::PostRequest);
        assert_eq!(result.ret_code, 3);
        assert_eq!(result.stdout, "");
        assert_eq!(result.stderr, "");
    }
}
