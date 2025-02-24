use std::io::{Result as IoResult, Write};
use anyhow_source_location::format_context;
use indicatif::TermLike;
use anyhow::Context;

#[derive(Debug)]
pub struct FileTerm{
    pub file: std::fs::File,
}

impl FileTerm {
    pub fn new(path: &str) -> anyhow::Result<Self> {
        let file = std::fs::File::create(path).context(format_context!("Failed to create file: {}", path))?;
        Ok(Self { file })
    }
}

impl Write for FileTerm {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.file.flush()
    }
}

// Implement TermLike for FileTerm
// These methods are not used in the markdown printer
impl TermLike for FileTerm {
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
        2048 // Return large width
    }

    fn height(&self) -> u16 {
        2048
    }

    fn flush(&self) -> std::io::Result<()> {
        Ok(()) // No-op flush
    }

    fn write_str(&self, _: &str) -> std::io::Result<()> {
        Ok(()) // Pretend everything is written successfully
    }

}

