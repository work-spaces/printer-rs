use indicatif::ProgressStyle;
use owo_colors::OwoColorize;
use serde::Serialize;
use std::{cell::RefCell, io::BufRead, sync::mpsc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Trace,
    Debug,
    Message,
    Info,
    Warning,
    Error,
    Silent,
}

const PROGRESS_PREFIX_WIDTH: usize = 0;

pub struct Section<'a, Context> {
    pub printer: &'a mut Printer<Context>,
}

impl<'a, Context> Section<'a, Context> {
    pub fn new(printer: &'a mut Printer<Context>, name: &str) -> anyhow::Result<Self> {
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

impl<Context> Drop for Section<'_, Context> {
    fn drop(&mut self) {
        self.printer.shift_left();
    }
}

pub struct ProgressBar {
    progress: indicatif::ProgressBar,
}

impl ProgressBar {
    fn new_multiprogress(total: Option<u64>, indent: usize) -> anyhow::Result<Self> {
        let progress = if let Some(total) = total {
            let progress = indicatif::ProgressBar::new(total);
            let template_string = {
                format!(
                    "{}{{prefix}} [{{bar:.cyan/blue}}] {{msg}}",
                    " ".repeat(indent)
                )
            };
            progress.set_style(
                ProgressStyle::with_template(template_string.as_str())
                    .unwrap()
                    .progress_chars("#>-"),
            );
            progress
        } else {
            let progress = indicatif::ProgressBar::new_spinner();
            let template_string =
                { format!("{}{{prefix}} {{spinner}} {{msg}}", " ".repeat(indent)) };
            progress.set_style(ProgressStyle::with_template(template_string.as_str()).unwrap());
            progress
        };
        Ok(Self { progress })
    }

    pub fn new(name: &str, total: Option<u64>, indent: usize) -> anyhow::Result<Self> {
        let progress = Self::new_multiprogress(total, indent)?;
        let prefix = format!("{name}:");
        progress.progress.set_prefix(
            format!("{prefix:width$}", width = PROGRESS_PREFIX_WIDTH)
                .bold()
                .to_string(),
        );

        Ok(Self {
            progress: progress.progress,
        })
    }

    pub fn set_message(&mut self, message: &str) {
        self.progress.set_message(message.to_owned());
    }

    pub fn increment(&mut self, count: u64) {
        self.progress.inc(count);
    }
}

pub struct MultiProgressBar {
    progress: indicatif::ProgressBar,
    ending_message: Option<String>,
}

impl MultiProgressBar {
    fn new(progress: indicatif::ProgressBar, ending_message: Option<String>) -> Self {
        Self {
            progress,
            ending_message,
        }
    }

    pub fn set_total(&mut self, total: u64) {
        self.progress.set_length(total);
    }

    pub fn set_prefix(&mut self, message: &str) {
        self.progress.set_prefix(message.to_owned());
    }

    pub fn set_message(&mut self, message: &str) {
        self.progress.set_message(message.to_owned());
    }

    pub fn increment(&mut self, count: u64) {
        self.progress.inc(count);
    }

    pub fn start_process(
        &mut self,
        command: &str,
        options: &ExecuteOptions,
    ) -> anyhow::Result<std::process::Child> {

        if let Some(directory) = &options.working_directory {
            if !std::path::Path::new(&directory).exists() {
                return Err(anyhow::anyhow!("Directory does not exist: {directory}"));
            }
        }

        let child_process = options.spawn(command)?;
        Ok(child_process)
    }
}

impl Drop for MultiProgressBar {
    fn drop(&mut self) {
        if let Some(message) = &self.ending_message {
            self.progress
                .finish_with_message(message.bold().to_string());
        } else {
            self.progress.finish_with_message("".to_string());
        }
    }
}

pub struct MultiProgress<'a, Context> {
    pub printer: &'a mut Printer<Context>,
    multi_progress: indicatif::MultiProgress,
}

impl<'a, Context> MultiProgress<'a, Context> {
    pub fn new(printer: &'a mut Printer<Context>) -> Self {
        Self {
            printer,
            multi_progress: indicatif::MultiProgress::new(),
        }
    }

    pub fn add_progress(&mut self, name: &str, total: Option<u64>) -> MultiProgressBar {
        let bar = ProgressBar::new_multiprogress(total, self.printer.indent())
            .expect("Internal Error: Failed to create progress bar");
        let progress = self.multi_progress.add(bar.progress);
        let ending_message = if total.is_none() {
            Some("Done!".to_string())
        } else {
            None
        };

        let prefix = format!("{name}:");
        progress.set_prefix(
            format!("{prefix:width$}", width = PROGRESS_PREFIX_WIDTH)
                .bold()
                .to_string(),
        );
        MultiProgressBar::new(progress, ending_message)
    }
}

pub struct Progress<'a, Context> {
    pub printer: &'a mut Printer<Context>,
    pub progress_bar: ProgressBar,
    ending_message: Option<String>,
}

impl<'a, Context> Progress<'a, Context> {
    pub fn new(
        printer: &'a mut Printer<Context>,
        name: &str,
        total: Option<u64>,
    ) -> anyhow::Result<Self> {
        let ending_message = if total.is_none() {
            Some("Done!".to_string())
        } else {
            None
        };

        let progress_bar = ProgressBar::new(name, total, printer.indent())?;

        Ok(Self {
            printer,
            progress_bar,
            ending_message,
        })
    }

    pub fn set_message(&mut self, message: &str) {
        self.progress_bar.set_message(message);
    }

    pub fn increment(&mut self, count: u64) {
        self.progress_bar.increment(count);
    }
}

impl<'a, Context> Drop for Progress<'a, Context> {
    fn drop(&mut self) {
        if let Some(message) = &self.ending_message {
            self.progress_bar
                .progress
                .finish_with_message(message.bold().to_string());
        } else {
            self.progress_bar.progress.finish();
        }
    }
}

pub struct Heading<'a, Context> {
    pub printer: &'a mut Printer<Context>,
}

impl<'a, Context> Heading<'a, Context> {
    pub fn new(printer: &'a mut Printer<Context>, name: &str) -> anyhow::Result<Self> {
        printer.newline()?;
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

impl<Context> Drop for Heading<'_, Context> {
    fn drop(&mut self) {
        self.printer.exit_heading();
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecuteOptions {
    pub label: String,
    pub working_directory: Option<String>,
    pub environment: Vec<(String, String)>,
    pub arguments: Vec<String>,
}

impl Default for ExecuteOptions {
    fn default() -> Self {
        Self {
            label: "working".to_string(),
            working_directory: None,
            environment: vec![],
            arguments: vec![],
        }
    }
}

impl ExecuteOptions {
    fn process_child_output<OutputType: std::io::Read + Send + 'static>(
        output: OutputType,
    ) -> anyhow::Result<(std::thread::JoinHandle<()>, mpsc::Receiver<String>)> {
        let (tx, rx) = mpsc::channel::<String>();

        let thread = std::thread::spawn(move || {
            use std::io::BufReader;
            let reader = BufReader::new(output);
            for line in reader.lines() {
                let line = line.unwrap();
                tx.send(line).unwrap();
            }
        });

        Ok((thread, rx))
    }

    fn spawn(&self, command: &str) -> anyhow::Result<std::process::Child> {
        use std::process::{Command, Stdio};
        let mut process = Command::new(command);

        for argument in &self.arguments {
            process.arg(argument);
        }

        if let Some(directory) = &self.working_directory {
            process.current_dir(directory);
        }

        for (key, value) in self.environment.iter() {
            process.env(key, value);
        }

        let result = process
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        Ok(result)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecuteLater {
    pub command: String,
    pub options: ExecuteOptions,
}

impl ExecuteLater {
    pub fn new(command: &str, options: ExecuteOptions) -> Self {
        Self {
            command: command.to_string(),
            options,
        }
    }

    pub fn to_string(&self) -> String {
        format!("{} {}", self.command, self.options.arguments.join(" "))
    }
}

pub struct ExecuteBatch {
    commands: Vec<(String, Vec<ExecuteLater>)>,
}

impl ExecuteBatch {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn add(&mut self, key: &str, options: Vec<ExecuteLater>) {
        for (name, execute_later) in self.commands.iter_mut() {
            if name == key {
                execute_later.extend(options);
                return;
            }
        }

        self.commands.push((key.to_string(), options));
    }

    /// Consumes the batch
    pub fn execute<'a, Context>(
        &mut self,
        printer: &'a mut Printer<Context>,
    ) -> anyhow::Result<()> {
        let section = Section::new(printer, &"Batch")?;

        let mut multi_progress = MultiProgress::new(section.printer);
        let mut handles = Vec::new();

        for (key, self_execute_later) in &self.commands {
            let mut progress = multi_progress.add_progress(key, None);
            progress.set_prefix(key);
            let mut execute_later = Vec::new();
            for execute in self_execute_later {
                execute_later.push(execute.clone());
            }

            let handle = std::thread::spawn(move || {
                for execute in execute_later {
                    progress.set_message(execute.to_string().as_str());

                    let child_process =
                        progress.start_process(execute.command.as_str(), &execute.options).expect("failed to start process");
                    let _ = monitor_process(child_process, &mut progress);
                }
                ()
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("Failed to join thread");
        }

        self.commands = Vec::new();
        Ok(())
    }
}

trait PrinterTrait: std::io::Write + indicatif::TermLike {}
impl<W: std::io::Write + indicatif::TermLike> PrinterTrait for W {}

pub struct Printer<Context> {
    pub is_dry_run: bool,
    pub level: Level,
    indent: RefCell<usize>,
    heading_count: usize,
    writer: RefCell<Box<dyn PrinterTrait>>,
    context: Context,
}

impl<Context> Printer<Context> {
    pub fn new_stdout(context: Context) -> Self {
        Self {
            indent: RefCell::new(0),
            level: Level::Info,
            heading_count: 0,
            writer: RefCell::new(Box::new(console::Term::stdout())),
            context,
            is_dry_run: false,
        }
    }

    pub fn context(&self) -> &Context {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    pub fn newline(&mut self) -> anyhow::Result<()> {
        let mut key_writer = self.writer.borrow_mut();
        writeln!(*key_writer, " ")?;
        Ok(())
    }

    pub fn trace<Type: Serialize>(&mut self, name: &str, value: &Type) -> anyhow::Result<()> {
        if self.level == Level::Trace {
            return Ok(());
        }
        return self.object(name, value);
    }

    pub fn debug<Type: Serialize>(&mut self, name: &str, value: &Type) -> anyhow::Result<()> {
        if self.level > Level::Debug {
            return Ok(());
        }
        return self.object(name, value);
    }

    pub fn message<Type: Serialize>(&mut self, name: &str, value: &Type) -> anyhow::Result<()> {
        if self.level > Level::Message {
            return Ok(());
        }
        return self.object(name, value);
    }

    pub fn info<Type: Serialize>(&mut self, name: &str, value: &Type) -> anyhow::Result<()> {
        if self.level > Level::Info {
            return Ok(());
        }
        return self.object(name, value);
    }

    pub fn warning<Type: Serialize>(&mut self, name: &str, value: &Type) -> anyhow::Result<()> {
        if self.level > Level::Warning {
            return Ok(());
        }
        return self.object(name.yellow().to_string().as_str(), value);
    }

    pub fn error<Type: Serialize>(&mut self, name: &str, value: &Type) -> anyhow::Result<()> {
        if self.level > Level::Error {
            return Ok(());
        }
        return self.object(name.red().to_string().as_str(), value);
    }

    pub fn code_block(&mut self, name: &str, content: &str) -> anyhow::Result<()> {
        let mut key_writer = self.writer.borrow_mut();

        writeln!(*key_writer, "```{name}")?;
        write!(*key_writer, "{}", content)?;
        writeln!(*key_writer, "```")?;

        Ok(())
    }

    fn object<Type: Serialize>(&mut self, name: &str, value: &Type) -> anyhow::Result<()> {
        let value = serde_json::to_value(&value).unwrap();

        if self.level <= Level::Message && value == serde_json::Value::Null {
            return Ok(());
        }

        {
            let mut key_writer = self.writer.borrow_mut();
            write!(
                *key_writer,
                "{}{}: ",
                " ".repeat(self.indent()),
                name.bold()
            )?;
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
                writeln!(self.writer.borrow_mut())?;
                self.shift_right();
                for (key, value) in map {
                    let is_skip = *value == serde_json::Value::Null && self.level > Level::Message;
                    if !is_skip {
                        {
                            let mut key_writer = self.writer.borrow_mut();
                            write!(*key_writer, "{}{}: ", " ".repeat(self.indent()), key.bold())?;
                        }
                        self.print_value(value)?;
                    }
                }
                self.shift_left();
            }
            serde_json::Value::Array(array) => {
                writeln!(self.writer.borrow_mut())?;
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

    pub fn start_process(
        &mut self,
        command: &str,
        options: &ExecuteOptions,
    ) -> anyhow::Result<std::process::Child> {
        let args = options.arguments.join(" ");
        let full_command = format!("{command} {args}");

        self.info("execute", &full_command)?;
        if let Some(directory) = &options.working_directory {
            self.info("directory", &directory)?;
            if !self.is_dry_run && !std::path::Path::new(&directory).exists() {
                return Err(anyhow::anyhow!("Directory does not exist: {directory}"));
            }
        }

        let child_process = options.spawn(command)?;
        Ok(child_process)
    }

    

    pub fn execute_process(
        &mut self,
        command: &str,
        options: &ExecuteOptions,
    ) -> anyhow::Result<()> {
        let section = Section::new(self, command)?;
        let child_process = section.printer.start_process(command, options)?;
        let mut multi_progress = MultiProgress::new(section.printer);
        let mut progress_bar = multi_progress.add_progress("progress", None);
        monitor_process(child_process, &mut progress_bar)?;

        Ok(())
    }
}

fn monitor_process(
    mut child_process: std::process::Child,
    progress_bar: &mut MultiProgressBar,
) -> anyhow::Result<()> {
    let child_stdout = child_process
        .stdout
        .take()
        .ok_or(anyhow::anyhow!("Internal Error: Child has no stdout"))?;

    let child_stderr = child_process
        .stderr
        .take()
        .ok_or(anyhow::anyhow!("Internal Error: Child has no stderr"))?;

    let (stdout_thread, stdout_rx) = ExecuteOptions::process_child_output(child_stdout)?;
    let (stderr_thread, stderr_rx) = ExecuteOptions::process_child_output(child_stderr)?;

    let handle_stdout =
        |progress: &mut MultiProgressBar, content: &mut String| -> anyhow::Result<()> {
            while let Ok(message) = stdout_rx.try_recv() {
                content.push_str(message.as_str());
                progress.set_message(message.as_str());
            }
            Ok(())
        };

    let handle_stderr =
        |progress: &mut MultiProgressBar, content: &mut String| -> anyhow::Result<()> {
            while let Ok(message) = stderr_rx.try_recv() {
                content.push_str(message.as_str());
                progress.set_message(message.as_str());
            }
            Ok(())
        };

    let exit_status;

    let mut stdout_content = String::new();
    let mut stderr_content = String::new();

    {
        loop {
            if let Ok(status) = child_process.try_wait() {
                if let Some(status) = status {
                    exit_status = Some(status);
                    break;
                }
            }

            handle_stdout(progress_bar, &mut stdout_content)?;
            handle_stderr(progress_bar, &mut stderr_content)?;
            std::thread::sleep(std::time::Duration::from_millis(50));
            progress_bar.increment(1);
        }
    }

    let _ = stdout_thread.join();
    let _ = stderr_thread.join();

    if let Some(exit_status) = exit_status {
        if !exit_status.success() {
            if let Some(code) = exit_status.code() {
                let exit_message = format!("Command failed with exit code: {code}");
                return Err(anyhow::anyhow!("{exit_message}"));
            } else {
                return Err(anyhow::anyhow!("Command failed with unknown exit code"));
            }
        }
    }

    Ok(())
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
        let mut printer: Printer<()> = Printer::new_stdout(());
        let mut options = ExecuteOptions::default();
        options.arguments.push("-alt".to_string());

        let runtime =
            tokio::runtime::Runtime::new().expect("Internal Error: Failed to create runtime");

        let (async_sender, sync_receiver) = flume::bounded(1);
        runtime.spawn(async move {
            async_sender.send_async(10).await.expect("Failed to send");
        });
        let received = sync_receiver.recv().expect("Failed to receive");

        drop(runtime);

        printer.info("Received", &received).unwrap();

        printer.execute_process("/bin/ls", &options).unwrap();

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
                let mut multi_progress = MultiProgress::new(&mut sub_section.printer);
                let mut first = multi_progress.add_progress("First", Some(10));
                let mut second = multi_progress.add_progress("Second", Some(50));
                let mut third = multi_progress.add_progress("Third", Some(100));

                let first_handle = std::thread::spawn(move || {
                    for index in 0..10 {
                        first.increment(1);
                        if index == 5 {
                            first.set_message("half way");
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                });

                let second_handle = std::thread::spawn(move || {
                    for index in 0..50 {
                        second.increment(1);
                        if index == 25 {
                            second.set_message("half way");
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                });

                for _ in 0..100 {
                    third.increment(1);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }

                first_handle.join().unwrap();
                second_handle.join().unwrap();
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
