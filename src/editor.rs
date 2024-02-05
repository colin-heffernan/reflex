#![warn(clippy::all, clippy::pedantic)]

use crate::{FileBuffer, Position, Terminal};
use crossterm::event::KeyCode;
use ropey::RopeSlice;
use std::{cmp, env, fmt};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Visual,
    Command,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Mode::Normal => write!(f, "NORMAL"),
            Mode::Insert => write!(f, "INSERT"),
            Mode::Visual => write!(f, "VISUAL"),
            Mode::Command => write!(f, "COMMAND"),
        }
    }
}

struct CommandLine {
    command: String,
    cursor_pos: usize,
    command_history: Vec<String>,
}

impl Default for CommandLine {
    /// Takes nothing.
    /// Builds a `CommandLine` to store commandline state.
    fn default() -> Self {
        Self {
            command: String::new(),
            cursor_pos: 0,
            command_history: Vec::new(),
        }
    }
}

pub struct Editor {
    should_quit: bool,
    terminal: Terminal,
    file_buffers: Vec<FileBuffer>,
    current_file_buffer_idx: usize,
    mode: Mode,
    command_line: CommandLine,
}

impl Default for Editor {
    /// Takes nothing.
    /// Builds an `Editor` to store program state.
    fn default() -> Self {
        let args: Vec<String> = env::args().collect();
        let default_buffer = if args.len() > 1 {
            let file_name = &args[1];
            let wrapped_doc = FileBuffer::open(file_name);
            if let Ok(document) = wrapped_doc {
                document
            } else {
                FileBuffer::default()
            }
        } else {
            FileBuffer::default()
        };
        Self {
            should_quit: false,
            terminal: Terminal::new().expect("Failed to initialize terminal"),
            file_buffers: vec![default_buffer],
            current_file_buffer_idx: 0,
            mode: Mode::default(),
            command_line: CommandLine::default(),
        }
    }
}

impl Editor {
    /// Takes itself.
    /// Runs the editor.
    pub fn run(&mut self) {
        loop {
            if let Err(error) = self.refresh_screen() {
                die(&error);
            }
            if self.should_quit {
                break;
            }
            if let Err(error) = self.process_keypress() {
                die(&error);
            }
        }
    }

    /// Takes itself.
    /// Redraws the screen.
    ///
    /// # Errors
    ///
    /// Returns an error if the `Terminal` cannot flush stdout.
    fn refresh_screen(&self) -> Result<(), std::io::Error> {
        Terminal::cursor_hide();
        Terminal::cursor_position(&Position::default());
        if self.should_quit {
            Terminal::cursor_show();
            Terminal::clear_screen();
            if self.terminal.alt_screen {
                Terminal::exit_alt_screen()?;
            }
            if self.terminal.raw_mode {
                Terminal::exit_raw_mode()?;
            }
            println!("Goodbye.\r");
        } else {
            self.draw_rows();
            self.draw_status_bar();
            if let Mode::Command = self.mode {
                self.draw_command_line();
                Terminal::cursor_position(&Position {
                    x: self.command_line.cursor_pos.saturating_add(1),
                    x_preferred: 0,
                    y: self.terminal.size().height.saturating_add(1) as usize,
                });
            } else {
                let file_buffer = &self.file_buffers[self.current_file_buffer_idx];
                Terminal::cursor_position(&file_buffer.get_primary_selection_cursor_pos());
            }
        }
        Terminal::cursor_show();
        Terminal::flush()
    }

    /// Takes itself.
    /// Draws the welcome message.
    fn draw_welcome_msg(&self) {
        let mut welcome_msg = format!("REFLEX -- v{VERSION}");
        let width = self.terminal.size().width as usize;
        let len = welcome_msg.len();
        let padding = width.saturating_sub(len) / 2;
        let spaces = " ".repeat(padding.saturating_sub(1));
        welcome_msg = format!("~{spaces}{welcome_msg}");
        welcome_msg.truncate(width);
        println!("{welcome_msg}\r");
    }

    /// Takes itself and a `RopeSlice`.
    /// Draws a single row of the editor.
    pub fn draw_row(&self, row: RopeSlice) {
        let file_buffer = &self.file_buffers[self.current_file_buffer_idx];
        let start = file_buffer.offset.x;
        let width = self.terminal.size().width as usize;
        let end = file_buffer.offset.x + width;
        let mut row_len = row.len_bytes();
        if row.slice(row_len.saturating_sub(1)..row_len).eq("\n") {
            row_len = row_len.saturating_sub(1);
        }
        let end = cmp::min(end, row_len);
        let start = cmp::min(start, end);
        let row = row.slice(start..end).to_string();
        println!("{row}\r");
    }

    /// Takes itself.
    /// Draws all of the text rows of the editor.
    fn draw_rows(&self) {
        let file_buffer = &self.file_buffers[self.current_file_buffer_idx];
        let height = match self.mode {
            Mode::Command => self.terminal.size().height.saturating_sub(1),
            _ => self.terminal.size().height,
        };
        for terminal_row in 0..height {
            Terminal::clear_current_line();
            if let Some(row) = file_buffer.row(terminal_row as usize + file_buffer.offset.y) {
                self.draw_row(row);
            } else if self.file_buffers[self.current_file_buffer_idx].buffer_is_empty
                && terminal_row == height / 3
            {
                self.draw_welcome_msg();
            } else {
                println!("~\r");
            }
        }
    }

    /// Takes itself.
    /// Forwards all keystrokes to the appropriate functions.
    ///
    /// # Errors
    ///
    /// Returns an error if the `Terminal` cannot read the event.
    fn process_keypress(&mut self) -> Result<(), std::io::Error> {
        let key_event = Terminal::read_event()?;
        match key_event.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.command_line.command = String::new();
            }
            KeyCode::Char(c) => match self.mode {
                Mode::Normal | Mode::Visual => match c {
                    ':' => self.mode = Mode::Command,
                    'i' => self.mode = Mode::Insert,
                    _ => (),
                },
                Mode::Insert => {
                    self.file_buffers[self.current_file_buffer_idx].insert(c);
                    self.file_buffers[self.current_file_buffer_idx]
                        .shift_viewport(self.terminal.size());
                }
                Mode::Command => {
                    self.command_line
                        .command
                        .insert(self.command_line.cursor_pos, c);
                    self.command_line.cursor_pos = self.command_line.cursor_pos.saturating_add(1);
                }
            },
            KeyCode::Enter => match self.mode {
                Mode::Insert => {
                    self.file_buffers[self.current_file_buffer_idx].insert('\n');
                    self.file_buffers[self.current_file_buffer_idx]
                        .shift_viewport(self.terminal.size());
                }
                Mode::Command => self.execute_command()?,
                _ => (),
            },
            KeyCode::Delete => match self.mode {
                Mode::Insert => {
                    self.file_buffers[self.current_file_buffer_idx].delete(false);
                    self.file_buffers[self.current_file_buffer_idx]
                        .shift_viewport(self.terminal.size());
                }
                Mode::Command => {
                    if self.command_line.cursor_pos < self.command_line.command.len() {
                        self.command_line
                            .command
                            .remove(self.command_line.cursor_pos);
                    }
                }
                _ => (),
            },
            KeyCode::Backspace => match self.mode {
                Mode::Insert => {
                    self.file_buffers[self.current_file_buffer_idx].delete(true);
                    self.file_buffers[self.current_file_buffer_idx]
                        .shift_viewport(self.terminal.size());
                }
                Mode::Command => {
                    if self.command_line.cursor_pos > 0 {
                        self.command_line
                            .command
                            .remove(self.command_line.cursor_pos.saturating_sub(1));
                        self.command_line.cursor_pos =
                            self.command_line.cursor_pos.saturating_sub(1);
                    }
                }
                _ => (),
            },
            KeyCode::Left => {
                if let Mode::Command = self.mode {
                    self.command_line.cursor_pos = self.command_line.cursor_pos.saturating_sub(1);
                } else {
                    self.file_buffers[self.current_file_buffer_idx].move_cursors(key_event.code);
                    self.file_buffers[self.current_file_buffer_idx]
                        .shift_viewport(self.terminal.size());
                }
            }
            KeyCode::Right => {
                if let Mode::Command = self.mode {
                    if self.command_line.cursor_pos < self.command_line.command.len() {
                        self.command_line.cursor_pos =
                            self.command_line.cursor_pos.saturating_add(1);
                    }
                } else {
                    self.file_buffers[self.current_file_buffer_idx].move_cursors(key_event.code);
                    self.file_buffers[self.current_file_buffer_idx]
                        .shift_viewport(self.terminal.size());
                }
            }
            KeyCode::Down | KeyCode::Up => {
                if let Mode::Command = self.mode {
                } else {
                    self.file_buffers[self.current_file_buffer_idx].move_cursors(key_event.code);
                    self.file_buffers[self.current_file_buffer_idx]
                        .shift_viewport(self.terminal.size());
                }
            }
            _ => (),
        }
        Ok(())
    }

    /// Takes itself.
    /// Draws the status bar underneath the text bars.
    fn draw_status_bar(&self) {
        let mut status;
        let width = self.terminal.size().width as usize;
        let mut file_name = "[No Name]".to_string();
        if let Some(name) = &self.file_buffers[self.current_file_buffer_idx].file_path {
            file_name = name.clone();
            file_name.truncate(20);
        }
        let mut dirty_status = String::new();
        if self.file_buffers[self.current_file_buffer_idx].file_is_dirty {
            dirty_status = String::from(" (Dirty)");
        }
        status = format!(
            "{file_name}{dirty_status} - {} lines",
            self.file_buffers[self.current_file_buffer_idx].len()
        );
        // status = format!("{status}"); // This line is kept in case formatting is needed later.
        status.truncate(width);
        Terminal::clear_current_line();
        match self.mode {
            Mode::Command => println!(" {} {status}\r", self.mode),
            _ => print!(" {} {status}", self.mode),
        }
    }

    /// Takes itself.
    /// Draws the commandline underneath the status bar.
    fn draw_command_line(&self) {
        Terminal::clear_current_line();
        print!(":{}", self.command_line.command);
    }

    /// Takes itself.
    /// Executes the command currently typed in the commandline.
    fn execute_command(&mut self) -> Result<(), std::io::Error> {
        match &self.command_line.command[..] {
            "q" => self.should_quit = true,
            "w" => self.file_buffers[self.current_file_buffer_idx].save()?,
            "wq" => {
                self.file_buffers[self.current_file_buffer_idx].save()?;
                self.should_quit = true;
            }
            _ => (),
        }
        self.command_line
            .command_history
            .push(self.command_line.command.clone());
        self.command_line.command = String::new();
        self.command_line.cursor_pos = 0;
        self.mode = Mode::Normal;
        Ok(())
    }
}

/// Takes an error.
/// Kills the program intentionally and displays the error.
fn die(e: &std::io::Error) {
    Terminal::clear_screen();
    panic!("{e}");
}
