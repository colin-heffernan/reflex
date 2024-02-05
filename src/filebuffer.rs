#![warn(clippy::all, clippy::pedantic)]
use crate::Size;
use crossterm::event::KeyCode;
use ropey::{Rope, RopeSlice};
use std::{
    cmp,
    fs::File,
    io::{BufReader, BufWriter},
};

#[derive(Default)]
pub struct Position {
    pub x: usize,
    pub x_preferred: usize,
    pub y: usize,
}

#[derive(Default)]
pub struct Selection {
    pub anchor: Position,
    pub cursor: Position,
}

pub struct FileBuffer {
    file_contents: Rope,
    pub file_path: Option<String>,
    pub buffer_is_empty: bool,
    pub file_is_dirty: bool,
    pub selections: Vec<Selection>,
    pub primary_selection_idx: usize,
    pub offset: Position,
}

impl Default for FileBuffer {
    /// Takes nothing.
    /// Builds an empty `FileBuffer`.
    fn default() -> Self {
        Self {
            file_contents: Rope::new(),
            file_path: None,
            buffer_is_empty: true,
            file_is_dirty: false,
            selections: vec![Selection::default()],
            primary_selection_idx: 0,
            offset: Position::default(),
        }
    }
}

impl FileBuffer {
    /// Takes a string slice represinting a file path.
    /// Builds a `FileBuffer` from the contents of the
    /// file at the given path, if one exists.
    ///
    /// # Errors
    ///
    /// Will return an error if the file cannot be read,
    /// or if a rope cannot be created from the file.
    pub fn open(file_path: &str) -> Result<Self, std::io::Error> {
        let file_contents = Rope::from_reader(BufReader::new(File::open(file_path)?))?;
        Ok(Self {
            // buffer_has_content: true,
            file_contents,
            file_path: Some(file_path.to_string()),
            buffer_is_empty: false,
            selections: vec![Selection::default()],
            ..Default::default()
        })
    }

    /// Takes itself and a usize representing the index of a row.
    /// Returns a `RopeSlice` wrapped in an Option if there is a
    /// row with the given index. Otherwise, returns None.
    #[must_use]
    pub fn row(&self, index: usize) -> Option<RopeSlice> {
        self.file_contents.get_line(index)
    }

    /// Takes itself.
    /// Returns a usize representing the length of the file in lines.
    #[must_use]
    pub fn len(&self) -> usize {
        self.file_contents.len_lines()
    }

    /// Takes itself.
    /// Returns a bool representing whether the file is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.file_contents.len_lines() == 0
    }

    /// Takes itself and a char.
    /// Inserts the char into the file at the given position.
    pub fn insert(&mut self, c: char) {
        for i in 0..self.selections.len() {
            let char_pos = self.file_contents.line_to_char(self.selections[i].cursor.y)
                + self.selections[i].cursor.x;
            if self.selections[i].cursor.y == self.len() {
                self.file_contents.insert(char_pos, &'\n'.to_string()[..]);
            }
            self.file_contents.insert(char_pos, &c.to_string()[..]);
            if c == '\n' {
                self.selections[i].cursor.x = 0;
                self.selections[i].cursor.y = self.selections[i].cursor.y.saturating_add(1);
            } else {
                self.selections[i].cursor.x = self.selections[i].cursor.x.saturating_add(1);
            }
            for j in i + 1..self.selections.len() {
                if self.selections[j].cursor.y == self.selections[i].cursor.y
                    && self.selections[j].cursor.x > self.selections[i].cursor.x
                {
                    self.selections[j].cursor.x = self.selections[j].cursor.x.saturating_add(1);
                    self.selections[j].cursor.x_preferred =
                        self.selections[j].cursor.x_preferred.saturating_add(1);
                }
                if self.selections[j].anchor.y == self.selections[i].cursor.y
                    && self.selections[j].anchor.x > self.selections[i].cursor.x
                {
                    self.selections[j].anchor.x = self.selections[j].anchor.x.saturating_add(1);
                }
            }
        }
        self.buffer_is_empty = false;
        self.file_is_dirty = true;
    }

    /// Takes itself and the position of the cursor.
    /// Deletes the character under the cursor.
    pub fn delete(&mut self, backspace: bool) {
        for i in 0..self.selections.len() {
            if (self.selections[i].cursor.y >= self.len() && !backspace)
                || (self.selections[i].cursor.x == 0
                    && self.selections[i].cursor.y == 0
                    && backspace)
            {
                return;
            }
            let mut char_pos = self.file_contents.line_to_char(self.selections[i].cursor.y)
                + self.selections[i].cursor.x;
            if backspace {
                char_pos = char_pos.saturating_sub(1);
            };
            let newline_deleted = self.file_contents.slice(char_pos..=char_pos).eq("\n");
            if newline_deleted && backspace {
                self.selections[i].cursor.y = self.selections[i].cursor.y.saturating_sub(1);
                self.selections[i].cursor.x =
                    if let Some(row) = self.row(self.selections[i].cursor.y) {
                        row.len_chars().saturating_sub(1)
                    } else {
                        0
                    }
            } else if backspace {
                self.selections[i].cursor.x = self.selections[i].cursor.x.saturating_sub(1);
            }
            self.file_contents.remove(char_pos..=char_pos);
            for j in i + 1..self.selections.len() {
                if self.selections[j].cursor.y == self.selections[i].cursor.y
                    && self.selections[j].cursor.x > self.selections[i].cursor.x
                {
                    self.selections[j].cursor.x -= 1;
                }
                if self.selections[j].anchor.y == self.selections[i].cursor.y
                    && self.selections[j].anchor.x > self.selections[i].cursor.x
                {
                    self.selections[j].anchor.x -= 1;
                }
            }
        }
        self.file_is_dirty = true;
    }

    /// Takes itself.
    /// Writes the contents to the file path, if it exists.
    ///
    /// # Errors
    ///
    /// Will return an error if the file cannot be opened
    /// or created, or if the rope cannot be written to it.
    pub fn save(&mut self) -> Result<(), std::io::Error> {
        if let Some(file_name) = &self.file_path {
            self.file_contents
                .write_to(BufWriter::new(File::create(file_name)?))?;
            self.file_is_dirty = false;
            Ok(())
        } else {
            // FIXME
            self.file_contents
                .write_to(BufWriter::new(File::create("")?))?;
            Ok(())
        }
    }

    /// Takes itself.
    /// Sets cursor `x` pos based on cursor `x_preferred` pos
    /// and row width.
    fn update_cursors_x_pos(&mut self) {
        for i in 0..self.selections.len() {
            self.selections[i].cursor.x = if let Some(row) = self.row(self.selections[i].cursor.y) {
                cmp::min(
                    self.selections[i].cursor.x_preferred,
                    row.len_chars().saturating_sub(1),
                )
            } else {
                0
            }
        }
    }

    /// Takes itself and the key entered.
    /// Moves each cursor if possible.
    pub fn move_cursors(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Up => {
                for i in 0..self.selections.len() {
                    self.selections[i].cursor.y = self.selections[i].cursor.y.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                for i in 0..self.selections.len() {
                    if self.selections[i].cursor.y < self.len().saturating_sub(1) {
                        self.selections[i].cursor.y = self.selections[i].cursor.y.saturating_add(1);
                    }
                }
            }
            KeyCode::Left => {
                for i in 0..self.selections.len() {
                    self.selections[i].cursor.x_preferred =
                        self.selections[i].cursor.x.saturating_sub(1);
                }
            }
            KeyCode::Right => {
                for i in 0..self.selections.len() {
                    if let Some(row) = self.row(self.selections[i].cursor.y) {
                        if self.selections[i].cursor.x < row.len_chars().saturating_sub(1) {
                            self.selections[i].cursor.x_preferred =
                                self.selections[i].cursor.x.saturating_add(1);
                        }
                    }
                }
            }
            _ => (),
        }
        self.update_cursors_x_pos();
        for i in 0..self.selections.len() {
            self.selections[i].anchor.x = self.selections[i].cursor.x;
            self.selections[i].anchor.y = self.selections[i].cursor.y;
        }
    }

    /// Takes itself.
    /// Returns the position of the primary cursor on the screen.
    #[must_use]
    pub fn get_primary_selection_cursor_pos(&self) -> Position {
        let primary_selection = &self.selections[self.primary_selection_idx];
        let Position {
            x,
            x_preferred: _,
            y,
        } = primary_selection.cursor;
        let x = x.saturating_sub(self.offset.x);
        let y = y.saturating_sub(self.offset.y);
        Position {
            x,
            x_preferred: 0,
            y,
        }
    }

    /// Takes itself and a position.
    /// Returns the position of the cursor on the screen.
    ///
    /// # Errors
    ///
    /// Returns `None` if the cursor is off-screen.
    #[must_use]
    pub fn get_screen_cursor_pos(&self, cursor: &Position, size: &Size) -> Option<Position> {
        let Position {
            x,
            x_preferred: _,
            y,
        } = cursor;
        if x < &self.offset.x
            || x >= &self.offset.x.saturating_add(size.width as usize)
            || y < &self.offset.y
            || y >= &self.offset.y.saturating_add(size.height as usize)
        {
            None
        } else {
            let x = x.saturating_sub(self.offset.x);
            let y = y.saturating_sub(self.offset.y);
            Some(Position {
                x,
                x_preferred: 0,
                y,
            })
        }
    }

    /// Takes itself and a `Position`.
    /// Returns the char under the cursor.
    #[must_use]
    pub fn get_char_under_cursor(&self, cursor: &Position) -> char {
        let line = self.row(cursor.y);
        if let Some(line) = line {
            let char = line.get_char(cursor.x);
            if let Some(char) = char {
                if char == '\n' {
                    ' '
                } else {
                    char
                }
            } else {
                ' '
            }
        } else {
            ' '
        }
    }

    /// Takes itself and the terminal size.
    /// Scrolls the viewport so that the primary selection
    /// is in view.
    pub fn shift_viewport(&mut self, size: &Size) {
        let Position {
            x,
            x_preferred: _,
            y,
        } = self.get_primary_selection_cursor_pos();
        if x >= size.width as usize {
            self.offset.x = self.selections[self.primary_selection_idx]
                .cursor
                .x
                .saturating_sub(size.width as usize)
                .saturating_add(1);
        } else if self.offset.x > self.selections[self.primary_selection_idx].cursor.x {
            self.offset.x = self.selections[self.primary_selection_idx].cursor.x;
        }
        if y >= size.height as usize {
            self.offset.y = self.selections[self.primary_selection_idx]
                .cursor
                .y
                .saturating_sub(size.height as usize)
                .saturating_add(1);
        } else if self.offset.y > self.selections[self.primary_selection_idx].cursor.y {
            self.offset.y = self.selections[self.primary_selection_idx].cursor.y;
        }
    }
}
