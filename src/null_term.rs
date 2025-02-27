use indicatif::TermLike;
use std::fmt::Debug;
use std::io::{Result as IoResult, Write};

#[derive(Default)]
pub struct NullTerm;

impl Debug for NullTerm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NullTerm")
    }
}

impl Write for NullTerm {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        Ok(buf.len()) // Pretend everything is written successfully
    }

    fn flush(&mut self) -> IoResult<()> {
        Ok(()) // No-op flush
    }
}

// Implement TermLike for NullTerm
impl TermLike for NullTerm {
    fn write_line(&self, _: &str) -> IoResult<()> {
        Ok(()) // Discard the line
    }

    fn clear_line(&self) -> IoResult<()> {
        Ok(()) // Do nothing for clearing the line
    }

    fn move_cursor_up(&self, _: usize) -> IoResult<()> {
        Ok(()) // Do nothing for moving the cursor up
    }

    fn move_cursor_down(&self, _: usize) -> IoResult<()> {
        Ok(()) // Do nothing for moving the cursor down
    }

    fn move_cursor_left(&self, _: usize) -> std::io::Result<()> {
        Ok(()) // Do nothing for moving the cursor left
    }

    fn move_cursor_right(&self, _: usize) -> std::io::Result<()> {
        Ok(()) // Do nothing for moving the cursor right
    }

    fn width(&self) -> u16 {
        128 // Return 0 width
    }

    fn height(&self) -> u16 {
        128
    }

    fn flush(&self) -> std::io::Result<()> {
        Ok(()) // No-op flush
    }

    fn write_str(&self, _: &str) -> std::io::Result<()> {
        Ok(()) // Pretend everything is written successfully
    }
}
