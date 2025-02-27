use crate::Printer;
use std::sync::Arc;

pub struct Markdown<'a> {
    pub printer: &'a mut Printer,
}

pub fn heading(level: u8, content: &str) -> String {
    format!("{} {}\n\n", "#".repeat(level as usize), content)
}

pub fn hline() -> &'static str {
    "\n---\n\n"
}

pub fn list(items: Vec<Arc<str>>) -> String {
    let mut result = String::new();
    for item in items {
        result.push_str(format!("- {}\n", item).as_str());
    }
    result.push('\n');
    result
}

pub fn list_item(level: u8, item: &str) -> String {
    let level = if level == 0 { 1_usize } else { level as usize };
    format!("{}- {}\n", " ".repeat(((level) - 1) * 2), item)
}

pub fn bold(content: &str) -> String {
    format!("**{}**", content)
}

pub fn hyperlink(show: &str, link: &str) -> String {
    format!("[{show}]({link})")
}

pub fn italic(content: &str) -> String {
    format!("*{}*", content)
}

pub fn strikethrough(content: &str) -> String {
    format!("~~{}~~", content)
}

pub fn code(content: &str) -> String {
    format!("`{}`", content)
}

pub fn code_block(code_type: &str, content: &str) -> String {
    format!("```{code_type}\n{}\n```\n", content)
}

pub fn paragraph(content: &str) -> String {
    format!("{}\n\n", content)
}

impl<'a> Markdown<'a> {
    pub fn new(printer: &'a mut Printer) -> Self {
        Markdown { printer }
    }

    pub fn heading(&mut self, level: u8, content: &str) -> anyhow::Result<()> {
        self.printer.write(&heading(level, content))?;
        Ok(())
    }

    pub fn write(&mut self, content: &str) -> anyhow::Result<()> {
        self.printer.write(content)
    }

    pub fn hline(&mut self) -> anyhow::Result<()> {
        self.printer.write(hline())
    }

    pub fn list(&mut self, items: Vec<Arc<str>>) -> anyhow::Result<()> {
        self.printer.write(&list(items))
    }

    pub fn list_item(&mut self, level: u8, item: &str) -> anyhow::Result<()> {
        self.printer.write(&list_item(level, item))
    }

    pub fn bold(&mut self, content: &str) -> anyhow::Result<()> {
        self.printer.write(&bold(content))?;
        Ok(())
    }

    pub fn hyperlink(&mut self, show: &str, link: &str) -> anyhow::Result<()> {
        self.printer.write(&hyperlink(show, link))?;
        Ok(())
    }

    pub fn italic(&mut self, content: &str) -> anyhow::Result<()> {
        self.printer.write(&italic(content))?;
        Ok(())
    }

    pub fn strikethrough(&mut self, content: &str) -> anyhow::Result<()> {
        self.printer.write(&strikethrough(content))?;
        Ok(())
    }

    pub fn code(&mut self, content: &str) -> anyhow::Result<()> {
        self.printer.write(&code(content))?;
        Ok(())
    }

    pub fn code_block(&mut self, code_type: &str, content: &str) -> anyhow::Result<()> {
        self.printer.write(&code_block(code_type, content))?;
        Ok(())
    }

    pub fn paragraph(&mut self, content: &str) -> anyhow::Result<()> {
        self.printer.write(&paragraph(content))?;
        Ok(())
    }
}
