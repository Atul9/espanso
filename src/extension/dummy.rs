/*
 * This file is part of espanso.
 *
 * Copyright (C) 2020 Federico Terzi
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

use serde_yaml::{Mapping, Value};

pub struct DummyExtension {}

impl DummyExtension {
    pub fn new() -> DummyExtension {
        DummyExtension {}
    }
}

impl super::Extension for DummyExtension {
    fn name(&self) -> String {
        String::from("dummy")
    }

    fn calculate(&self, params: &Mapping, _: &Vec<String>) -> Option<String> {
        let echo = params.get(&Value::from("echo"));

        if let Some(echo) = echo {
            Some(echo.as_str().unwrap_or_default().to_owned())
        } else {
            None
        }
    }
}
