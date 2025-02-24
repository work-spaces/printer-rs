use crate::Printer;

pub struct Markdown<'a> {
    pub printer: &'a mut Printer,
}

impl<'a> Markdown<'a> {
    pub fn new(printer: &'a mut Printer) -> Self {
        Markdown { printer }
    }

    pub fn heading(&mut self, level: u8, content: &str) -> anyhow::Result<()> {
        self.printer
            .write(&format!("{} {}\n\n", "#".repeat(level as usize), content))?;
        Ok(())
    }

    pub fn write(&mut self, content: &str) -> anyhow::Result<()> {
        self.printer.write(content)?;
        Ok(())
    }

    pub fn hline(&mut self) -> anyhow::Result<()> {
        self.printer.write("\n---\n\n")?;
        Ok(())
    }

    pub fn list(&mut self, items: Vec<&str>) -> anyhow::Result<()> {
        for item in items {
            self.printer.write(&format!("- {}\n", item))?;
        }
        self.printer.write("\n")?;
        Ok(())
    }

    pub fn list_item(&mut self, level: u8, item: &str) -> anyhow::Result<()> {
        let level = if level == 0 { 1_usize } else { level as usize };
        self.printer
            .write(&format!("{}- {}\n", " ".repeat(((level) - 1) * 2), item))?;
        Ok(())
    }

    pub fn bold(&mut self, content: &str) -> anyhow::Result<()> {
        self.printer.write(&format!("**{}**", content))?;
        Ok(())
    }

    pub fn get_link(show: &str, link: &str) -> String {
        format!("[{show}]({link})")
    }

    pub fn link(&mut self, show: &str, link: &str) -> anyhow::Result<()> {
        self.printer.write(&Self::get_link(show, link))?;
        Ok(())
    }

    pub fn italic(&mut self, content: &str) -> anyhow::Result<()> {
        self.printer.write(&format!("*{}*", content))?;
        Ok(())
    }

    pub fn strikethrough(&mut self, content: &str) -> anyhow::Result<()> {
        self.printer.write(&format!("~~{}~~", content))?;
        Ok(())
    }

    pub fn code(&mut self, content: &str) -> anyhow::Result<()> {
        self.printer.write(&format!("`{}`", content))?;
        Ok(())
    }

    pub fn code_block(&mut self, code_type: &str, content: &str) -> anyhow::Result<()> {
        self.printer
            .write(&format!("```{code_type}\n{}\n```\n", content))?;
        Ok(())
    }

    pub fn paragraph(&mut self, content: &str) -> anyhow::Result<()> {
        self.printer.write(&format!("{}\n\n", content))?;
        Ok(())
    }

}
