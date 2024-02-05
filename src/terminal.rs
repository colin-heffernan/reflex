#![warn(clippy::all, clippy::pedantic)]
use crate::Position;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{read, Event, KeyEvent},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use std::io::{self, Write};

pub struct Size {
    pub width: u16,
    pub height: u16,
}

pub struct Terminal {
    size: Size,
    pub raw_mode: bool,
    pub alt_screen: bool,
}

impl Terminal {
    /// Takes nothing.
    /// Creates a new `Terminal`.
    ///
    /// # Errors
    ///
    /// Will return an error if the terminal abstraction
    /// cannot be created.
    pub fn new() -> Result<Self, std::io::Error> {
        let size = crossterm::terminal::size()?;
        let raw_ok = Self::enter_raw_mode();
        let alt_ok = Self::enter_alt_screen();
        Ok(Self {
            size: Size {
                width: size.0,
                height: size.1.saturating_sub(1),
            },
            raw_mode: raw_ok.is_ok(),
            alt_screen: alt_ok.is_ok(),
        })
    }

    /// Takes itself.
    /// Returns the terminal's size.
    #[must_use]
    pub fn size(&self) -> &Size {
        &self.size
    }

    /// Takes nothing.
    /// Clears the terminal screen.
    pub fn clear_screen() {
        print!("{}", Clear(crossterm::terminal::ClearType::All));
    }

    /// Takes nothing.
    /// Enters the alternate screen.
    ///
    /// # Errors
    ///
    /// Will return an error if the alternate screen
    /// cannot be entered.
    pub fn enter_alt_screen() -> io::Result<()> {
        execute!(io::stdout(), EnterAlternateScreen)
    }

    /// Takes nothing.
    /// Exits the alternate screen.
    ///
    /// # Errors
    ///
    /// Will return an error if the alternate screen
    /// cannot be exited.
    pub fn exit_alt_screen() -> io::Result<()> {
        execute!(io::stdout(), LeaveAlternateScreen)
    }

    /// Takes nothing.
    /// Enters raw mode.
    ///
    /// # Errors
    ///
    /// Will return an error if raw mode
    /// cannot be entered.
    pub fn enter_raw_mode() -> io::Result<()> {
        enable_raw_mode()
    }

    /// Takes nothing.
    /// Exits the alternate screen.
    ///
    /// # Errors
    ///
    /// Will return an error if the alternate screen
    /// cannot be exited.
    pub fn exit_raw_mode() -> io::Result<()> {
        disable_raw_mode()
    }

    /// Takes nothing.
    /// Clears the line on the terminal that the cursor is on.
    pub fn clear_current_line() {
        print!("{}", Clear(crossterm::terminal::ClearType::CurrentLine));
    }

    /// Takes a Position.
    /// Moves the cursor to the Position.
    #[allow(clippy::cast_possible_truncation)]
    pub fn cursor_position(position: &Position) {
        let Position {
            x,
            x_preferred: _,
            y,
        } = position;
        let x = *x as u16;
        let y = *y as u16;
        print!("{}", MoveTo(x, y));
    }

    /// Takes nothing.
    /// Flushes stdout.
    ///
    /// # Errors
    ///
    /// Will return an error if stdout cannot be flushed.
    pub fn flush() -> Result<(), std::io::Error> {
        io::stdout().flush()
    }

    /// Takes nothing.
    /// Hides the cursor.
    pub fn cursor_hide() {
        print!("{Hide}");
    }

    /// Takes nothing.
    /// Shows the cursor.
    pub fn cursor_show() {
        print!("{Show}");
    }

    /// Takes nothing.
    /// Returns a `KeyEvent`.
    ///
    /// # Errors
    ///
    /// Will return an error if the event cannot be read.
    pub fn read_event() -> Result<KeyEvent, std::io::Error> {
        loop {
            if let Event::Key(event) = read()? {
                return Ok(event);
            }
        }
    }
}
