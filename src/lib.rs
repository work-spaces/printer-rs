use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use serde::Serialize;
use std::cell::RefCell;

pub struct Section<'a> {
    pub printer: &'a mut Printer,
}

impl<'a> Section<'a> {
    pub fn new(printer: &'a mut Printer, name: &str) -> anyhow::Result<Self> {
        {
            let mut key_writer = printer.writer.borrow_mut();
            writeln!(
                *key_writer,
                "{}{}:",
                " ".repeat(printer.indent()),
                name.bold()
            )?;
        }
        printer.shift_right();
        Ok(Self { printer })
    }
}

impl Drop for Section<'_> {
    fn drop(&mut self) {
        self.printer.shift_left();
    }
}

pub struct Progress<'a> {
    pub printer: &'a mut Printer,
    progress: ProgressBar,
    ending_message: Option<String>,
}

impl<'a> Progress<'a> {
    pub fn new(printer: &'a mut Printer, name: &str, total: Option<u64>) -> anyhow::Result<Self> {
        let ending_message = if total.is_none() {
            Some(format!("{name}: Done!"))
        } else {
            None
        };

        let progress = if let Some(total) = total {
            let progress = ProgressBar::new(total);
            let template_string = {
                format!(
                    "{}{{msg}} [{{bar:.cyan/blue}}]",
                    " ".repeat(printer.indent())
                )
            };
            progress.set_style(
                ProgressStyle::with_template(template_string.as_str())
                    .unwrap()
                    .progress_chars("#>-"),
            );
            progress
        } else {
            let progress = ProgressBar::new_spinner();
            let template_string =
                { format!("{}{{msg}} {{spinner}}", " ".repeat(printer.indent())) };
            progress.set_style(ProgressStyle::with_template(template_string.as_str()).unwrap());
            progress
        };
        progress.set_message(format!("{name}:").bold().to_string());

        Ok(Self {
            printer,
            progress,
            ending_message,
        })
    }

    pub fn increment(&mut self, count: u64) {
        self.progress.inc(count);
    }
}

impl Drop for Progress<'_> {
    fn drop(&mut self) {
        if let Some(message) = &self.ending_message {
            self.progress.finish_with_message(message.to_owned());
        } else {
            self.progress.finish();
        }
    }
}

pub struct Heading<'a> {
    pub printer: &'a mut Printer,
}

impl<'a> Heading<'a> {
    pub fn new(printer: &'a mut Printer, name: &str) -> anyhow::Result<Self> {
        printer.enter_heading();
        {
            let mut key_writer = printer.writer.borrow_mut();
            let heading = if printer.heading_count == 1 {
                format!("{} {name}", "#".repeat(printer.heading_count))
                    .yellow()
                    .bold()
                    .to_string()
            } else {
                format!("{} {name}", "#".repeat(printer.heading_count))
                    .bold()
                    .to_string()
            };
            writeln!(*key_writer, "{heading}")?;
            writeln!(*key_writer)?;
        }
        Ok(Self { printer })
    }
}

impl Drop for Heading<'_> {
    fn drop(&mut self) {
        self.printer.exit_heading();
    }
}

trait PrinterTrait: std::io::Write + indicatif::TermLike {}
impl<W: std::io::Write + indicatif::TermLike> PrinterTrait for W {}

pub struct Printer {
    indent: RefCell<usize>,
    heading_count: usize,
    writer: RefCell<Box<dyn PrinterTrait>>,
}

impl Printer {
    pub fn new_stdout() -> Self {
        Self {
            indent: RefCell::new(0),
            heading_count: 0,
            writer: RefCell::new(Box::new(console::Term::stdout())),
        }
    }

    pub fn newline(&mut self) -> anyhow::Result<()> {
        let mut key_writer = self.writer.borrow_mut();
        writeln!(*key_writer)?;
        Ok(())
    }

    pub fn object<Type: Serialize>(&mut self, name: &str, value: &Type) -> anyhow::Result<()> {
        let value = serde_json::to_value(&value).unwrap();

        {
            let mut key_writer = self.writer.borrow_mut();
            if value.is_object() || value.is_array() {
                writeln!(*key_writer, "{}{}:", " ".repeat(self.indent()), name.bold())?;
            } else {
                write!(
                    *key_writer,
                    "{}{}: ",
                    " ".repeat(self.indent()),
                    name.bold()
                )?;
            }
        }

        self.print_value(&value)?;
        Ok(())
    }

    fn indent(&self) -> usize {
        *self.indent.borrow()
    }

    fn enter_heading(&mut self) {
        self.heading_count += 1;
    }

    fn exit_heading(&mut self) {
        self.heading_count -= 1;
    }

    fn shift_right(&self) {
        *self.indent.borrow_mut() += 2;
    }

    fn shift_left(&self) {
        *self.indent.borrow_mut() -= 2;
    }

    fn print_value(&self, value: &serde_json::Value) -> anyhow::Result<()> {
        match value {
            serde_json::Value::Object(map) => {
                self.shift_right();
                for (key, value) in map {
                    {
                        let mut key_writer = self.writer.borrow_mut();
                        write!(*key_writer, "{}{}: ", " ".repeat(self.indent()), key.bold())?;
                    }
                    self.print_value(value)?;
                }
                self.shift_left();
            }
            serde_json::Value::Array(array) => {
                self.shift_right();
                for (index, value) in array.iter().enumerate() {
                    {
                        let mut key_writer = self.writer.borrow_mut();
                        write!(*key_writer, "{}[{index}]: ", " ".repeat(self.indent()))?;
                    }
                    self.print_value(value)?;
                }
                self.shift_left();
            }
            serde_json::Value::Null => {
                let mut key_writer = self.writer.borrow_mut();
                writeln!(*key_writer, "null")?;
            }
            serde_json::Value::Bool(value) => {
                let mut key_writer = self.writer.borrow_mut();
                writeln!(*key_writer, "{value}")?;
            }
            serde_json::Value::Number(value) => {
                let mut key_writer = self.writer.borrow_mut();
                writeln!(*key_writer, "{value}")?;
            }
            serde_json::Value::String(value) => {
                let mut key_writer = self.writer.borrow_mut();
                writeln!(*key_writer, "{value}")?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    pub struct Test {
        pub name: String,
        pub age: u32,
        pub alive: bool,
        pub dead: bool,
        pub children: f64,
    }

    #[test]
    fn printer() {
        let mut printer = Printer::new_stdout();

        {
            let mut heading = Heading::new(&mut printer, "First").unwrap();
            {
                let section = Section::new(&mut heading.printer, "PersonWrapper").unwrap();
                section
                    .printer
                    .object(
                        "Person",
                        &Test {
                            name: "John".to_string(),
                            age: 30,
                            alive: true,
                            dead: false,
                            children: 2.5,
                        },
                    )
                    .unwrap();
            }

            let mut sub_heading = Heading::new(&mut heading.printer, "Second").unwrap();

            let mut sub_section = Section::new(&mut sub_heading.printer, "PersonWrapper").unwrap();
            sub_section.printer.object("Hello", &"World").unwrap();

            {
                let mut progress =
                    Progress::new(&mut sub_section.printer, "Progressing", Some(10)).unwrap();

                for _ in 0..10 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    progress.increment(1);
                }
            }

            {
                let mut spinner =
                    Progress::new(&mut sub_section.printer, "Spinning", None).unwrap();

                for _ in 0..10 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    spinner.increment(1);
                }
            }
        }
    }
}
