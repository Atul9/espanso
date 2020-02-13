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

use crate::clipboard::ClipboardManager;
use crate::config::BackendType;
use crate::config::ConfigManager;
use crate::event::{ActionEventReceiver, ActionType};
use crate::extension::Extension;
use crate::keyboard::KeyboardManager;
use crate::matcher::{Match, MatchContentType, MatchReceiver};
use crate::render::{RenderResult, Renderer};
use crate::ui::{MenuItem, MenuItemType, UIManager};
use log::{error, info, warn};
use regex::{Captures, Regex};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::exit;
use std::time::SystemTime;

pub struct Engine<
    'a,
    S: KeyboardManager,
    C: ClipboardManager,
    M: ConfigManager<'a>,
    U: UIManager,
    R: Renderer,
> {
    keyboard_manager: &'a S,
    clipboard_manager: &'a C,
    config_manager: &'a M,
    ui_manager: &'a U,
    renderer: &'a R,

    enabled: RefCell<bool>,
    last_action_time: RefCell<SystemTime>, // Used to block espanso from re-interpreting it's own inputs
    action_noop_interval: u128,
}

impl<
        'a,
        S: KeyboardManager,
        C: ClipboardManager,
        M: ConfigManager<'a>,
        U: UIManager,
        R: Renderer,
    > Engine<'a, S, C, M, U, R>
{
    pub fn new(
        keyboard_manager: &'a S,
        clipboard_manager: &'a C,
        config_manager: &'a M,
        ui_manager: &'a U,
        renderer: &'a R,
    ) -> Engine<'a, S, C, M, U, R> {
        let enabled = RefCell::new(true);
        let last_action_time = RefCell::new(SystemTime::now());
        let action_noop_interval = config_manager.default_config().action_noop_interval;

        Engine {
            keyboard_manager,
            clipboard_manager,
            config_manager,
            ui_manager,
            renderer,
            enabled,
            last_action_time,
            action_noop_interval,
        }
    }

    fn build_menu(&self) -> Vec<MenuItem> {
        let mut menu = Vec::new();

        let enabled = self.enabled.borrow();
        let toggle_text = if *enabled { "Disable" } else { "Enable" }.to_owned();
        menu.push(MenuItem {
            item_type: MenuItemType::Button,
            item_name: toggle_text,
            item_id: ActionType::Toggle as i32,
        });

        menu.push(MenuItem {
            item_type: MenuItemType::Separator,
            item_name: "".to_owned(),
            item_id: 999,
        });

        menu.push(MenuItem {
            item_type: MenuItemType::Button,
            item_name: "Exit".to_owned(),
            item_id: ActionType::Exit as i32,
        });

        menu
    }

    fn return_content_if_preserve_clipboard_is_enabled(&self) -> Option<String> {
        // If the preserve_clipboard option is enabled, first save the current
        // clipboard content in order to restore it later.
        if self.config_manager.default_config().preserve_clipboard {
            match self.clipboard_manager.get_clipboard() {
                Some(clipboard) => Some(clipboard),
                None => None,
            }
        } else {
            None
        }
    }

    /// Used to check if the last action has been executed within a specified interval.
    /// If so, return true (blocking the action), otherwise false.
    fn check_last_action_and_set(&self, interval: u128) -> bool {
        let mut last_action_time = self.last_action_time.borrow_mut();
        if let Ok(elapsed) = last_action_time.elapsed() {
            if elapsed.as_millis() < interval {
                return true;
            }
        }

        (*last_action_time) = SystemTime::now();
        return false;
    }
}

lazy_static! {
    static ref VAR_REGEX: Regex = Regex::new("\\{\\{\\s*(?P<name>\\w+)\\s*\\}\\}").unwrap();
}

impl<
        'a,
        S: KeyboardManager,
        C: ClipboardManager,
        M: ConfigManager<'a>,
        U: UIManager,
        R: Renderer,
    > MatchReceiver for Engine<'a, S, C, M, U, R>
{
    fn on_match(&self, m: &Match, trailing_separator: Option<char>) {
        let config = self.config_manager.active_config();

        if !config.enable_active {
            return;
        }

        // avoid espanso reinterpreting its own actions
        if self.check_last_action_and_set(self.action_noop_interval) {
            return;
        }

        let char_count = if trailing_separator.is_none() {
            m.trigger.chars().count() as i32
        } else {
            m.trigger.chars().count() as i32 + 1 // Count also the separator
        };

        self.keyboard_manager.delete_string(char_count);

        let mut previous_clipboard_content: Option<String> = None;

        let rendered = self.renderer.render_match(m, config, vec![]);

        match rendered {
            RenderResult::Text(mut target_string) => {
                // If a trailing separator was counted in the match, add it back to the target string
                if let Some(trailing_separator) = trailing_separator {
                    if trailing_separator == '\r' {
                        // If the trailing separator is a carriage return,
                        target_string.push('\n'); // convert it to new line
                    } else {
                        target_string.push(trailing_separator);
                    }
                }

                // Convert Windows style newlines into unix styles
                target_string = target_string.replace("\r\n", "\n");

                // Calculate cursor rewind moves if a Cursor Hint is present
                let index = target_string.find("$|$");
                let cursor_rewind = if let Some(index) = index {
                    // Convert the byte index to a char index
                    let char_str = &target_string[0..index];
                    let char_index = char_str.chars().count();
                    let total_size = target_string.chars().count();

                    // Remove the $|$ placeholder
                    target_string = target_string.replace("$|$", "");

                    // Calculate the amount of rewind moves needed (LEFT ARROW).
                    // Subtract also 3, equal to the number of chars of the placeholder "$|$"
                    let moves = (total_size - char_index - 3) as i32;
                    Some(moves)
                } else {
                    None
                };

                match config.backend {
                    BackendType::Inject => {
                        // Send the expected string. On linux, newlines are managed automatically
                        // while on windows and macos, we need to emulate a Enter key press.

                        if cfg!(target_os = "linux") {
                            self.keyboard_manager.send_string(&target_string);
                        } else {
                            // To handle newlines, substitute each "\n" char with an Enter key press.
                            let splits = target_string.split('\n');

                            for (i, split) in splits.enumerate() {
                                if i > 0 {
                                    self.keyboard_manager.send_enter();
                                }

                                self.keyboard_manager.send_string(split);
                            }
                        }
                    }
                    BackendType::Clipboard => {
                        // If the preserve_clipboard option is enabled, save the current
                        // clipboard content to restore it later.
                        previous_clipboard_content =
                            self.return_content_if_preserve_clipboard_is_enabled();

                        self.clipboard_manager.set_clipboard(&target_string);
                        self.keyboard_manager.trigger_paste(&config.paste_shortcut);
                    }
                }

                if let Some(moves) = cursor_rewind {
                    // Simulate left arrow key presses to bring the cursor into the desired position
                    self.keyboard_manager.move_cursor_left(moves);
                }
            }
            RenderResult::Image(image_path) => {
                // If the preserve_clipboard option is enabled, save the current
                // clipboard content to restore it later.
                previous_clipboard_content = self.return_content_if_preserve_clipboard_is_enabled();

                self.clipboard_manager.set_clipboard_image(&image_path);
                self.keyboard_manager.trigger_paste(&config.paste_shortcut);
            }
            RenderResult::Error => {
                error!("Could not render match: {}", m.trigger);
            }
        }

        // Restore previous clipboard content
        if let Some(previous_clipboard_content) = previous_clipboard_content {
            // Sometimes an expansion gets overwritten before pasting by the previous content
            // A delay is needed to mitigate the problem
            std::thread::sleep(std::time::Duration::from_millis(
                config.restore_clipboard_delay as u64,
            ));

            self.clipboard_manager
                .set_clipboard(&previous_clipboard_content);
        }
    }

    fn on_enable_update(&self, status: bool) {
        // avoid espanso reinterpreting its own actions
        if self.check_last_action_and_set(self.action_noop_interval) {
            return;
        }

        let message = if status {
            "espanso enabled"
        } else {
            "espanso disabled"
        };

        info!("Toggled: {}", message);

        let mut enabled_ref = self.enabled.borrow_mut();
        *enabled_ref = status;

        self.ui_manager.notify(message);
    }

    fn on_passive(&self) {
        // avoid espanso reinterpreting its own actions
        if self.check_last_action_and_set(self.action_noop_interval) {
            return;
        }

        let config = self.config_manager.active_config();

        if !config.enable_passive {
            return;
        }

        info!("Passive mode activated");

        // Trigger a copy shortcut to transfer the content of the selection to the clipboard
        self.keyboard_manager.trigger_copy();

        // Sleep for a while, giving time to effectively copy the text
        std::thread::sleep(std::time::Duration::from_millis(100)); // TODO: avoid hardcoding

        // Then get the text from the clipboard and render the match output
        let clipboard = self.clipboard_manager.get_clipboard();

        if let Some(clipboard) = clipboard {
            let rendered = self.renderer.render_passive(&clipboard, &config);

            match rendered {
                RenderResult::Text(payload) => {
                    // Paste back the result in the field
                    self.clipboard_manager.set_clipboard(&payload);

                    std::thread::sleep(std::time::Duration::from_millis(100)); // TODO: avoid hardcoding
                    self.keyboard_manager.trigger_paste(&config.paste_shortcut);
                }
                _ => warn!("Cannot expand passive match"),
            }
        }
    }
}

impl<
        'a,
        S: KeyboardManager,
        C: ClipboardManager,
        M: ConfigManager<'a>,
        U: UIManager,
        R: Renderer,
    > ActionEventReceiver for Engine<'a, S, C, M, U, R>
{
    fn on_action_event(&self, e: ActionType) {
        match e {
            ActionType::IconClick => {
                self.ui_manager.show_menu(self.build_menu());
            }
            ActionType::Exit => {
                info!("Terminating espanso.");
                self.ui_manager.cleanup();
                exit(0);
            }
            _ => {}
        }
    }
}
