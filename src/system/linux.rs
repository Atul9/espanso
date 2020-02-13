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

use std::os::raw::c_char;

use crate::bridge::linux::{
    get_active_window_class, get_active_window_executable, get_active_window_name,
};
use std::ffi::CStr;

pub struct LinuxSystemManager {}

impl super::SystemManager for LinuxSystemManager {
    fn get_current_window_title(&self) -> Option<String> {
        unsafe {
            let mut buffer: [c_char; 100] = [0; 100];
            let res = get_active_window_name(buffer.as_mut_ptr(), buffer.len() as i32);

            if res > 0 {
                let c_string = CStr::from_ptr(buffer.as_ptr());

                let string = c_string.to_str();
                if let Ok(string) = string {
                    return Some((*string).to_owned());
                }
            }
        }

        None
    }

    fn get_current_window_class(&self) -> Option<String> {
        unsafe {
            let mut buffer: [c_char; 100] = [0; 100];
            let res = get_active_window_class(buffer.as_mut_ptr(), buffer.len() as i32);

            if res > 0 {
                let c_string = CStr::from_ptr(buffer.as_ptr());

                let string = c_string.to_str();
                if let Ok(string) = string {
                    return Some((*string).to_owned());
                }
            }
        }

        None
    }

    fn get_current_window_executable(&self) -> Option<String> {
        unsafe {
            let mut buffer: [c_char; 100] = [0; 100];
            let res = get_active_window_executable(buffer.as_mut_ptr(), buffer.len() as i32);

            if res > 0 {
                let c_string = CStr::from_ptr(buffer.as_ptr());

                let string = c_string.to_str();
                if let Ok(string) = string {
                    return Some((*string).to_owned());
                }
            }
        }

        None
    }
}

impl LinuxSystemManager {
    pub fn new() -> LinuxSystemManager {
        LinuxSystemManager {}
    }
}
