use anyhow::Context;
use anyhow_source_location::{format_context, format_error};
use indicatif::ProgressStyle;
use owo_colors::OwoColorize;
use serde::Serialize;
use std::{
    io::{BufRead, Write},
    sync::{mpsc, Arc, Mutex},
};
use strum::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display)]
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

fn format_log(indent: usize, max_width: usize, level: Level, message: &str) -> String {
    let mut result = format!(
        "{}{}: {message}",
        " ".repeat(indent),
        level.to_string().bold()
    );
    while result.len() < max_width {
        result.push(' ');
    }
    result.push('\n');
    result
}

pub struct Section<'a> {
    pub printer: &'a mut Printer,
}

impl<'a> Section<'a> {
    pub fn new(printer: &'a mut Printer, name: &str) -> anyhow::Result<Self> {
        printer
            .write(format!("{}{}:", " ".repeat(printer.indent), name.bold()).as_str())
            .context(format_context!(""))?;
        printer.shift_right();
        Ok(Self { printer })
    }
}

impl Drop for Section<'_> {
    fn drop(&mut self) {
        self.printer.shift_left();
    }
}

pub struct MultiProgressBar {
    lock: Arc<Mutex<()>>,
    printer_level: Level,
    indent: usize,
    max_width: usize,
    progress_width: usize,
    progress: indicatif::ProgressBar,
    final_message: Option<String>,
}

impl MultiProgressBar {
    pub fn total(&self) -> Option<u64> {
        self.progress.length()
    }

    pub fn set_total(&mut self, total: u64) {
        if let Some(length) = self.progress.length() {
            if length != total {
                let _lock = self.lock.lock().unwrap();
                self.progress.set_length(total);
                self.progress.set_position(0);
            }
        }
    }

    pub fn log(&mut self, level: Level, message: &str) {
        if level >= self.printer_level {
            let _lock = self.lock.lock().unwrap();
            self.progress
                .println(format_log(self.indent, self.max_width, level, message).as_str());
        }
    }

    pub fn set_prefix(&mut self, message: &str) {
        let _lock = self.lock.lock().unwrap();
        self.progress.set_prefix(message.to_owned());
    }

    fn construct_message(&self, message: &str) -> String {
        let prefix_size = self.progress.prefix().len();
        sanitize_output(message, self.max_width - self.progress_width - prefix_size)
    }

    pub fn set_message(&mut self, message: &str) {
        let _lock = self.lock.lock().unwrap();
        self.progress.set_message(self.construct_message(message));
    }

    pub fn set_ending_message(&mut self, message: &str) {
        self.final_message = Some(self.construct_message(message));
    }

    pub fn increment_with_overflow(&mut self, count: u64) {
        let _lock = self.lock.lock().unwrap();
        self.progress.inc(count);
        if let Some(total) = self.total() {
            if self.progress.position() >= total {
                self.progress.set_position(0);
            }
        }
    }

    pub fn increment(&mut self, count: u64) {
        let _lock = self.lock.lock().unwrap();
        self.progress.inc(count);
    }

    fn start_process(
        &mut self,
        command: &str,
        options: &ExecuteOptions,
    ) -> anyhow::Result<std::process::Child> {
        if let Some(directory) = &options.working_directory {
            if !std::path::Path::new(&directory).exists() {
                return Err(format_error!("Directory does not exist: {directory}"));
            }
        }

        let child_process = options.spawn(command).context(format_context!(""))?;
        Ok(child_process)
    }

    pub fn execute_process(
        &mut self,
        command: &str,
        options: ExecuteOptions,
    ) -> anyhow::Result<Option<String>> {
        self.set_message(&options.get_full_command(command));
        let child_process = self
            .start_process(command, &options)
            .context(format_context!("Failed to start process {command}"))?;
        let result =
            monitor_process(command, child_process, self, &options).context(format_context!(""))?;
        Ok(result)
    }
}

impl Drop for MultiProgressBar {
    fn drop(&mut self) {
        if let Some(message) = &self.final_message {
            let _lock = self.lock.lock().unwrap();
            self.progress
                .finish_with_message(self.construct_message(message).bold().to_string());
        }
    }
}

pub struct MultiProgress<'a> {
    pub printer: &'a mut Printer,
    multi_progress: indicatif::MultiProgress,
}

impl<'a> MultiProgress<'a> {
    pub fn new(printer: &'a mut Printer) -> Self {
        let locker = printer.lock.clone();
        let _lock = locker.lock().unwrap();

        Self {
            printer,
            multi_progress: indicatif::MultiProgress::new(),
        }
    }

    pub fn add_progress(
        &mut self,
        prefix: &str,
        total: Option<u64>,
        finish_message: Option<&str>,
    ) -> MultiProgressBar {
        let _lock = self.printer.lock.lock().unwrap();

        let indent = self.printer.indent;
        let progress = if let Some(total) = total {
            let progress = indicatif::ProgressBar::new(total);
            let template_string =
                { format!("{}[{{bar:.cyan/blue}}] {{prefix}} {{msg}}", " ".repeat(0)) };
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

        let progress = self.multi_progress.add(progress);

        let prefix = format!("{prefix}:");
        progress.set_prefix(
            format!("{prefix:width$}", width = PROGRESS_PREFIX_WIDTH)
                .bold()
                .to_string(),
        );
        MultiProgressBar {
            lock: self.printer.lock.clone(),
            printer_level: self.printer.level,
            indent: self.printer.indent,
            progress,
            progress_width: 28, // This is the default from indicatif?
            max_width: self.printer.max_width,
            final_message: finish_message.map(|s| s.to_string()),
        }
    }
}

pub struct Heading<'a> {
    pub printer: &'a mut Printer,
}

impl<'a> Heading<'a> {
    pub fn new(printer: &'a mut Printer, name: &str) -> anyhow::Result<Self> {
        printer.newline().context(format_context!(""))?;
        printer.enter_heading();
        {
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
            printer
                .write(heading.as_str())
                .context(format_context!(""))?;
            printer.write("\n").context(format_context!(""))?;
        }
        Ok(Self { printer })
    }
}

impl Drop for Heading<'_> {
    fn drop(&mut self) {
        self.printer.exit_heading();
    }
}

#[derive(Clone, Debug)]
pub struct ExecuteOptions {
    pub label: String,
    pub is_return_stdout: bool,
    pub working_directory: Option<String>,
    pub environment: Vec<(String, String)>,
    pub arguments: Vec<String>,
    pub log_file_path: Option<String>,
}

impl Default for ExecuteOptions {
    fn default() -> Self {
        Self {
            label: "working".to_string(),
            is_return_stdout: false,
            working_directory: None,
            environment: vec![],
            arguments: vec![],
            log_file_path: None,
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
            .spawn()
            .context(format_context!("{command}"))?;

        Ok(result)
    }

    pub fn get_full_command(&self, command: &str) -> String {
        format!("{command} {}", self.arguments.join(" "))
    }

    pub fn get_full_command_in_working_directory(&self, command: &str) -> String {
        format!(
            "{} {command} {}",
            if let Some(directory) = &self.working_directory {
                directory
            } else {
                ""
            },
            self.arguments.join(" "),
        )
    }
}

trait PrinterTrait: std::io::Write + indicatif::TermLike {}
impl<W: std::io::Write + indicatif::TermLike> PrinterTrait for W {}

pub struct Printer {
    pub level: Level,
    lock: Arc<Mutex<()>>,
    indent: usize,
    heading_count: usize,
    max_width: usize,
    writer: Box<dyn PrinterTrait>,
}

impl Printer {
    pub fn new_stdout() -> Self {
        let mut max_width = 80;
        if let Some((width, _)) = term_size::dimensions() {
            max_width = width - 1;
        }
        Self {
            indent: 0,
            lock: Arc::new(Mutex::new(())),
            level: Level::Info,
            heading_count: 0,
            max_width,
            writer: Box::new(console::Term::stdout()),
        }
    }

    fn write(&mut self, message: &str) -> anyhow::Result<()> {
        let _lock = self.lock.lock().unwrap();
        write!(self.writer, "{}", message).context(format_context!(""))?;
        Ok(())
    }

    pub fn newline(&mut self) -> anyhow::Result<()> {
        self.write("\n")?;
        Ok(())
    }

    pub fn trace<Type: Serialize>(&mut self, name: &str, value: &Type) -> anyhow::Result<()> {
        if self.level == Level::Trace {
            return Ok(());
        }
        self.object(name, value)
    }

    pub fn debug<Type: Serialize>(&mut self, name: &str, value: &Type) -> anyhow::Result<()> {
        if self.level > Level::Debug {
            return Ok(());
        }
        self.object(name, value)
    }

    pub fn message<Type: Serialize>(&mut self, name: &str, value: &Type) -> anyhow::Result<()> {
        if self.level > Level::Message {
            return Ok(());
        }
        self.object(name, value)
    }

    pub fn info<Type: Serialize>(&mut self, name: &str, value: &Type) -> anyhow::Result<()> {
        if self.level > Level::Info {
            return Ok(());
        }
        self.object(name, value)
    }

    pub fn warning<Type: Serialize>(&mut self, name: &str, value: &Type) -> anyhow::Result<()> {
        if self.level > Level::Warning {
            return Ok(());
        }
        self.object(name.yellow().to_string().as_str(), value)
    }

    pub fn error<Type: Serialize>(&mut self, name: &str, value: &Type) -> anyhow::Result<()> {
        if self.level > Level::Error {
            return Ok(());
        }
        self.object(name.red().to_string().as_str(), value)
    }

    pub fn log(&mut self, level: Level, message: &str) -> anyhow::Result<()> {
        if self.level > level {
            return Ok(());
        }
        self.write(format_log(self.indent, self.max_width, level, message).as_str())
    }

    pub fn code_block(&mut self, name: &str, content: &str) -> anyhow::Result<()> {
        self.write(format!("```{name}\n{content}```\n").as_str())
            .context(format_context!(""))?;
        Ok(())
    }

    fn object<Type: Serialize>(&mut self, name: &str, value: &Type) -> anyhow::Result<()> {
        let value = serde_json::to_value(value).context(format_context!(""))?;

        if self.level <= Level::Message && value == serde_json::Value::Null {
            return Ok(());
        }

        self.write(format!("{}{}: ", " ".repeat(self.indent), name.bold()).as_str())?;

        self.print_value(&value).context(format_context!(""))?;
        Ok(())
    }

    fn enter_heading(&mut self) {
        self.heading_count += 1;
    }

    fn exit_heading(&mut self) {
        self.heading_count -= 1;
    }

    fn shift_right(&mut self) {
        self.indent += 2;
    }

    fn shift_left(&mut self) {
        self.indent -= 2;
    }

    fn print_value(&mut self, value: &serde_json::Value) -> anyhow::Result<()> {
        match value {
            serde_json::Value::Object(map) => {
                self.write("\n").context(format_context!(""))?;
                self.shift_right();
                for (key, value) in map {
                    let is_skip = *value == serde_json::Value::Null && self.level > Level::Message;
                    if !is_skip {
                        {
                            self.write(
                                format!("{}{}: ", " ".repeat(self.indent), key.bold()).as_str(),
                            )
                            .context(format_context!(""))?;
                        }
                        self.print_value(value).context(format_context!(""))?;
                    }
                }
                self.shift_left();
            }
            serde_json::Value::Array(array) => {
                self.write("\n").context(format_context!(""))?;
                self.shift_right();
                for (index, value) in array.iter().enumerate() {
                    self.write(format!("{}[{index}]: ", " ".repeat(self.indent)).as_str())?;
                    self.print_value(value).context(format_context!(""))?;
                }
                self.shift_left();
            }
            serde_json::Value::Null => {
                self.write("null\n").context(format_context!(""))?;
            }
            serde_json::Value::Bool(value) => {
                self.write(format!("{value}\n").as_str())
                    .context(format_context!(""))?;
            }
            serde_json::Value::Number(value) => {
                self.write(format!("{value}\n").as_str())
                    .context(format_context!(""))?;
            }
            serde_json::Value::String(value) => {
                self.write(format!("{value}\n").as_str())
                    .context(format_context!(""))?;
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

        self.info("execute", &full_command)
            .context(format_context!(""))?;
        if let Some(directory) = &options.working_directory {
            self.info("directory", &directory)
                .context(format_context!(""))?;
            if !std::path::Path::new(&directory).exists() {
                return Err(format_error!("Directory does not exist: {directory}"));
            }
        }

        let child_process = options
            .spawn(command)
            .context(format_context!("{command}"))?;
        Ok(child_process)
    }

    pub fn execute_process(
        &mut self,
        command: &str,
        options: ExecuteOptions,
    ) -> anyhow::Result<Option<String>> {
        let section = Section::new(self, command).context(format_context!(""))?;
        let child_process = section
            .printer
            .start_process(command, &options)
            .context(format_context!("{command}"))?;
        let mut multi_progress = MultiProgress::new(section.printer);
        let mut progress_bar = multi_progress.add_progress("progress", None, None);
        let result = monitor_process(command, child_process, &mut progress_bar, &options)
            .context(format_context!(""))?;

        Ok(result)
    }
}

fn sanitize_output(input: &str, max_length: usize) -> String {
    //remove all backspaces and truncate
    const EXCLUDED: &[char] = &[8u8 as char, '\r', '\n'];

    let mut result = String::new();
    for (offset, character) in input.chars().enumerate() {
        if offset < max_length && !EXCLUDED.contains(&character) {
            result.push(character);
        }
    }
    while result.len() < max_length {
        result.push(' ');
    }

    result
}

fn monitor_process(
    command: &str,
    mut child_process: std::process::Child,
    progress_bar: &mut MultiProgressBar,
    options: &ExecuteOptions,
) -> anyhow::Result<Option<String>> {
    let child_stdout = child_process
        .stdout
        .take()
        .ok_or(format_error!("Internal Error: Child has no stdout"))?;

    let child_stderr = child_process
        .stderr
        .take()
        .ok_or(format_error!("Internal Error: Child has no stderr"))?;

    let (stdout_thread, stdout_rx) = ExecuteOptions::process_child_output(child_stdout)?;
    let (stderr_thread, stderr_rx) = ExecuteOptions::process_child_output(child_stderr)?;

    let handle_stdout = |progress: &mut MultiProgressBar,
                         writer: Option<&mut std::fs::File>,
                         content: Option<&mut String>|
     -> anyhow::Result<()> {
        let mut stdout = String::new();
        while let Ok(message) = stdout_rx.try_recv() {
            if writer.is_some() || content.is_some() {
                stdout.push_str(message.as_str());
                stdout.push('\n');
            }
            progress.set_message(message.as_str());
        }

        if let Some(content) = content {
            content.push_str(stdout.as_str());
        }

        if let Some(writer) = writer {
            let _ = writer.write_all(stdout.as_bytes());
        }
        Ok(())
    };

    let handle_stderr = |progress: &mut MultiProgressBar,
                         writer: Option<&mut std::fs::File>,
                         content: &mut String|
     -> anyhow::Result<()> {
        let mut stderr = String::new();
        while let Ok(message) = stderr_rx.try_recv() {
            stderr.push_str(message.as_str());
            stderr.push('\n');
            progress.set_message(message.as_str());
        }
        content.push_str(stderr.as_str());
        if let Some(writer) = writer {
            let _ = writer.write_all(stderr.as_bytes());
        }
        Ok(())
    };

    let exit_status;

    let mut stderr_content = String::new();
    let mut stdout_content = String::new();

    let mut output_file = if let Some(log_path) = options.log_file_path.as_ref() {
        let mut file = std::fs::File::create(log_path.as_str())
            .context(format_context!("while creating {log_path}"))?;

        let command = format!("command: {}\n", command);
        let working_directory = format!(
            "directory: {}\n",
            options.working_directory.as_deref().unwrap_or("")
        );
        let arguments = format!("arguments: {}\n\n", options.arguments.join(" "));

        file.write(format!("{command}{working_directory}{arguments}").as_bytes())
            .context(format_context!("while writing {log_path}"))?;

        Some(file)
    } else {
        None
    };

    loop {
        if let Ok(Some(status)) = child_process.try_wait() {
            exit_status = Some(status);
            break;
        }

        let stdout_content = if options.is_return_stdout {
            Some(&mut stdout_content)
        } else {
            None
        };

        handle_stdout(progress_bar, output_file.as_mut(), stdout_content)
            .context(format_context!("failed to handle stdout"))?;
        handle_stderr(progress_bar, output_file.as_mut(), &mut stderr_content)
            .context(format_context!("failed to handle stderr"))?;
        std::thread::sleep(std::time::Duration::from_millis(100));
        progress_bar.increment_with_overflow(1);
    }

    let _ = stdout_thread.join();
    let _ = stderr_thread.join();

    {
        let stdout_content = if options.is_return_stdout {
            Some(&mut stdout_content)
        } else {
            None
        };

        handle_stdout(progress_bar, output_file.as_mut(), stdout_content)
            .context(format_context!("while handling stdout"))?;
    }

    handle_stderr(progress_bar, output_file.as_mut(), &mut stderr_content)
        .context(format_context!("while handling stderr"))?;

    if let Some(exit_status) = exit_status {
        if !exit_status.success() {
            if let Some(code) = exit_status.code() {
                let exit_message = format!("Command failed with exit code: {code}");
                return Err(format_error!("{exit_message} : {stderr_content}"));
            } else {
                return Err(format_error!(
                    "Command failed with unknown exit code: {stderr_content}"
                ));
            }
        }
    }

    Ok(if options.is_return_stdout {
        Some(stdout_content)
    } else {
        None
    })
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

        printer.execute_process("/bin/ls", options).unwrap();

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
                let mut multi_progress = MultiProgress::new(&mut sub_section.printer);
                let mut first = multi_progress.add_progress("First", Some(10), None);
                let mut second = multi_progress.add_progress("Second", Some(50), None);
                let mut third = multi_progress.add_progress("Third", Some(100), None);

                let first_handle = std::thread::spawn(move || {
                    first.set_ending_message("Done!");
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
        }

        {
            let runtime =
                tokio::runtime::Runtime::new().expect("Internal Error: Failed to create runtime");

            let heading = Heading::new(&mut printer, "Async").unwrap();

            let mut multi_progress = MultiProgress::new(heading.printer);

            let mut handles = Vec::new();

            let task1_progress = multi_progress.add_progress("Task1", Some(30), None);
            let task2_progress = multi_progress.add_progress("Task2", Some(30), None);
            let task1 = async move {
                let mut progress = task1_progress;
                progress.set_message("Task1a");
                for _ in 0..10 {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    progress.increment(1);
                }

                progress.set_message("Task1b");
                for _ in 0..10 {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    progress.increment(1);
                }

                progress.set_message("Task1c");
                for _ in 0..10 {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    progress.increment(1);
                }
                ()
            };
            handles.push(runtime.spawn(task1));

            let task2 = async move {
                let mut progress = task2_progress;
                progress.set_message("Task2a");
                for _ in 0..10 {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    progress.increment(1);
                }

                progress.set_message("Task2b");
                for _ in 0..10 {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    progress.increment(1);
                }

                progress.set_message("Task2c");
                for _ in 0..10 {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    progress.increment(1);
                }
                ()
            };
            handles.push(runtime.spawn(task2));

            for handle in handles {
                runtime.block_on(handle).unwrap();
            }
        }
    }
}
