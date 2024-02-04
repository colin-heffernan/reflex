#![warn(clippy::all, clippy::pedantic)]
mod editor;
mod filebuffer;
mod terminal;

use editor::Editor;
pub use editor::Mode;
pub use filebuffer::FileBuffer;
pub use filebuffer::Position;
pub use terminal::Size;
pub use terminal::Terminal;

fn main() {
    Editor::default().run();
}
