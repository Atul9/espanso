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
use rand::seq::SliceRandom;
use serde_yaml::{Mapping, Value};

pub struct RandomExtension {}

impl RandomExtension {
    pub fn new() -> RandomExtension {
        RandomExtension {}
    }
}

impl super::Extension for RandomExtension {
    fn name(&self) -> String {
        String::from("random")
    }

    fn calculate(&self, params: &Mapping, args: &Vec<String>) -> Option<String> {
        let choices = params.get(&Value::from("choices"));
        if choices.is_none() {
            warn!("No 'choices' parameter specified for random variable");
            return None;
        }
        let choices = choices.unwrap().as_sequence();
        if let Some(choices) = choices {
            let str_choices = choices
                .iter()
                .map(|arg| arg.as_str().unwrap_or_default().to_string())
                .collect::<Vec<String>>();

            // Select a random choice between the possibilities
            let choice = str_choices.choose(&mut rand::thread_rng());

            match choice {
                Some(output) => {
                    // Render arguments
                    let output = crate::render::utils::render_args(output, args);

                    return Some(output);
                }
                None => {
                    error!("Could not select a random choice.");
                    return None;
                }
            }
        }

        error!("choices array have an invalid format '{:?}'", choices);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::Extension;

    #[test]
    fn test_random_basic() {
        let mut params = Mapping::new();
        let choices = vec!["first", "second", "third"];
        params.insert(Value::from("choices"), Value::from(choices.clone()));

        let extension = RandomExtension::new();
        let output = extension.calculate(&params, &vec![]);

        assert!(output.is_some());

        let output = output.unwrap();

        assert!(choices.iter().any(|x| x == &output));
    }

    #[test]
    fn test_random_with_args() {
        let mut params = Mapping::new();
        let choices = vec!["first $0$", "second $0$", "$0$ third"];
        params.insert(Value::from("choices"), Value::from(choices.clone()));

        let extension = RandomExtension::new();
        let output = extension.calculate(&params, &vec!["test".to_owned()]);

        assert!(output.is_some());

        let output = output.unwrap();

        let rendered_choices = vec!["first test", "second test", "test third"];

        assert!(rendered_choices.iter().any(|x| x == &output));
    }
}
