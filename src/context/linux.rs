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

use crate::bridge::linux::*;
use crate::event::KeyModifier::*;
use crate::event::*;
use log::{error, info};
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::process::exit;
use std::sync::mpsc::Sender;
use std::{thread, time};

#[repr(C)]
pub struct LinuxContext {
    pub send_channel: Sender<Event>,
}

impl LinuxContext {
    pub fn new(send_channel: Sender<Event>) -> Box<LinuxContext> {
        // Check if the X11 context is available
        let x11_available = unsafe { check_x11() };

        if x11_available < 0 {
            error!("Error, can't connect to X11 context");
            std::process::exit(100);
        }

        let context = Box::new(LinuxContext { send_channel });

        unsafe {
            let context_ptr = &*context as *const LinuxContext as *const c_void;

            register_keypress_callback(keypress_callback);

            let res = initialize(context_ptr);
            if res <= 0 {
                error!("Could not initialize linux context, error: {}", res);
                exit(10);
            }
        }

        context
    }
}

impl super::Context for LinuxContext {
    fn eventloop(&self) {
        unsafe {
            eventloop();
        }
    }
}

impl Drop for LinuxContext {
    fn drop(&mut self) {
        unsafe {
            cleanup();
        }
    }
}

// Native bridge code

extern "C" fn keypress_callback(
    _self: *mut c_void,
    raw_buffer: *const u8,
    len: i32,
    is_modifier: i32,
    key_code: i32,
) {
    unsafe {
        let _self = _self as *mut LinuxContext;

        if is_modifier == 0 {
            // Char event
            // Convert the received buffer to a string
            let c_str = CStr::from_ptr(raw_buffer as (*const c_char));
            let char_str = c_str.to_str();

            // Send the char through the channel
            match char_str {
                Ok(char_str) => {
                    let event = Event::Key(KeyEvent::Char(char_str.to_owned()));
                    (*_self).send_channel.send(event).unwrap();
                }
                Err(e) => {
                    error!("Unable to receive char: {}", e);
                }
            }
        } else {
            // Modifier event
            let modifier: Option<KeyModifier> = match key_code {
                133 => Some(META),
                50 => Some(SHIFT),
                64 => Some(ALT),
                37 => Some(CTRL),
                22 => Some(BACKSPACE),
                _ => None,
            };

            if let Some(modifier) = modifier {
                let event = Event::Key(KeyEvent::Modifier(modifier));
                (*_self).send_channel.send(event).unwrap();
            }
        }
    }
}
