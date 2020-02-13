/*
 * This file is part of espanso.
 *
 * Copyright (C) 2019 Federico Terzi
 *
 * espanso is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * espanso is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with espanso.  If not, see <https://www.gnu.org/licenses/>.
 */

use log::{error, warn};
use regex::{Captures, Regex};
use serde_yaml::{Mapping, Value};
use std::process::Command;

lazy_static! {
    static ref POS_ARG_REGEX: Regex = if cfg!(target_os = "windows") {
        Regex::new("%(?P<pos>\\d+)").unwrap()
    } else {
        Regex::new("\\$(?P<pos>\\d+)").unwrap()
    };
}

pub struct ShellExtension {}

impl ShellExtension {
    pub fn new() -> ShellExtension {
        ShellExtension {}
    }
}

impl super::Extension for ShellExtension {
    fn name(&self) -> String {
        String::from("shell")
    }

    fn calculate(&self, params: &Mapping, args: &Vec<String>) -> Option<String> {
        let cmd = params.get(&Value::from("cmd"));
        if cmd.is_none() {
            warn!("No 'cmd' parameter specified for shell variable");
            return None;
        }
        let cmd = cmd.unwrap().as_str().unwrap();

        // Render positional parameters in args
        let cmd = POS_ARG_REGEX
            .replace_all(&cmd, |caps: &Captures| {
                let position_str = caps.name("pos").unwrap().as_str();
                let position = position_str.parse::<i32>().unwrap_or(-1);
                if position >= 0 && position < args.len() as i32 {
                    args[position as usize].to_owned()
                } else {
                    "".to_owned()
                }
            })
            .to_string();

        let output = if cfg!(target_os = "windows") {
            Command::new("cmd").args(&["/C", &cmd]).output()
        } else {
            Command::new("sh").arg("-c").arg(&cmd).output()
        };

        match output {
            Ok(output) => {
                let output_str = String::from_utf8_lossy(output.stdout.as_slice());
                let mut output_str = output_str.into_owned();

                // If specified, trim the output
                let trim_opt = params.get(&Value::from("trim"));
                if let Some(value) = trim_opt {
                    let val = value.as_bool();
                    if let Some(val) = val {
                        if val {
                            output_str = output_str.trim().to_owned()
                        }
                    }
                }

                Some(output_str)
            }
            Err(e) => {
                error!("Could not execute cmd '{}', error: {}", cmd, e);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::Extension;

    #[test]
    fn test_shell_basic() {
        let mut params = Mapping::new();
        params.insert(Value::from("cmd"), Value::from("echo hello world"));

        let extension = ShellExtension::new();
        let output = extension.calculate(&params, &vec![]);

        assert!(output.is_some());

        if cfg!(target_os = "windows") {
            assert_eq!(output.unwrap(), "hello world\r\n");
        } else {
            assert_eq!(output.unwrap(), "hello world\n");
        }
    }

    #[test]
    fn test_shell_trimmed() {
        let mut params = Mapping::new();
        params.insert(Value::from("cmd"), Value::from("echo hello world"));
        params.insert(Value::from("trim"), Value::from(true));

        let extension = ShellExtension::new();
        let output = extension.calculate(&params, &vec![]);

        assert!(output.is_some());
        assert_eq!(output.unwrap(), "hello world");
    }

    #[test]
    fn test_shell_trimmed_2() {
        let mut params = Mapping::new();
        if cfg!(target_os = "windows") {
            params.insert(Value::from("cmd"), Value::from("echo    hello world     "));
        } else {
            params.insert(
                Value::from("cmd"),
                Value::from("echo \"   hello world     \""),
            );
        }

        params.insert(Value::from("trim"), Value::from(true));

        let extension = ShellExtension::new();
        let output = extension.calculate(&params, &vec![]);

        assert!(output.is_some());
        assert_eq!(output.unwrap(), "hello world");
    }

    #[test]
    fn test_shell_trimmed_malformed() {
        let mut params = Mapping::new();
        params.insert(Value::from("cmd"), Value::from("echo hello world"));
        params.insert(Value::from("trim"), Value::from("error"));

        let extension = ShellExtension::new();
        let output = extension.calculate(&params, &vec![]);

        assert!(output.is_some());
        if cfg!(target_os = "windows") {
            assert_eq!(output.unwrap(), "hello world\r\n");
        } else {
            assert_eq!(output.unwrap(), "hello world\n");
        }
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_shell_pipes() {
        let mut params = Mapping::new();
        params.insert(Value::from("cmd"), Value::from("echo hello world | cat"));
        params.insert(Value::from("trim"), Value::from(true));

        let extension = ShellExtension::new();
        let output = extension.calculate(&params, &vec![]);

        assert!(output.is_some());
        assert_eq!(output.unwrap(), "hello world");
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_shell_args_unix() {
        let mut params = Mapping::new();
        params.insert(Value::from("cmd"), Value::from("echo $0"));

        let extension = ShellExtension::new();
        let output = extension.calculate(&params, &vec!["hello".to_owned()]);

        assert!(output.is_some());

        assert_eq!(output.unwrap(), "hello\n");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_shell_args_windows() {
        let mut params = Mapping::new();
        params.insert(Value::from("cmd"), Value::from("echo %0"));

        let extension = ShellExtension::new();
        let output = extension.calculate(&params, &vec!["hello".to_owned()]);

        assert!(output.is_some());

        assert_eq!(output.unwrap(), "hello\r\n");
    }
}
