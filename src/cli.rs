use std::{fs, io, panic};
use std::fs::File;
use std::io::{ErrorKind, Write};
use std::panic::PanicInfo;
use std::path::{Path, PathBuf};
use std::process::{Command, exit, Output};
use std::time::Instant;

use atty::Stream;
use clap::CommandFactory;
use clap::FromArgMatches;
use clap::Parser as CliParser;
use regex::bytes::Regex;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

use crate::core::{Error, Parser};

/// A list of paths and contents to copy into the build directory
const OUT_DIR_FILES: &[(&str, &[u8])] = &[
    ("build/runtime/mod.rs", include_bytes!("runtime/mod.rs")),
    (
        "build/runtime/context.rs",
        include_bytes!("runtime/context.rs"),
    ),
    (
        "build/runtime/grammar.rs",
        include_bytes!("runtime/grammar.rs"),
    ),
    ("build/runtime/input.rs", include_bytes!("runtime/input.rs")),
    (
        "build/runtime/result.rs",
        include_bytes!("runtime/result.rs"),
    ),
    (
        "build/runtime/buffered_iter.rs",
        include_bytes!("runtime/buffered_iter.rs"),
    ),
    ("build/harness.rs", include_bytes!("include/harness.rs")),
    ("build/loader.js", include_bytes!("include/loader.js")),
    ("loader.d.ts", include_bytes!("include/loader.d.ts")),
    (".gitignore", include_bytes!("include/gitignore")),
];

pub fn run() {
    let command = (Cli::command() as clap::Command).color(clap::ColorChoice::Auto);
    let cli: Cli = Cli::from_arg_matches(&command.get_matches()).unwrap();

    let context = Context::new(cli);
    context.run();
}

/// Installs a nicer panic that tells the user about the crash before printing
/// the usual backtrace
pub fn setup_panic_hook() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| panic_hook(info, &default_hook)));
}

fn panic_hook(info: &PanicInfo, default_hook: &dyn Fn(&PanicInfo)) {
    let color = if atty::is(Stream::Stderr) {
        termcolor::ColorChoice::Auto
    } else {
        termcolor::ColorChoice::Never
    };

    let mut red = ColorSpec::new();
    red.set_fg(Some(Color::Red));

    let mut reset = ColorSpec::new();
    reset.set_reset(true);

    let mut stderr = StandardStream::stderr(color);
    let _ = stderr.set_color(&red);
    let _ = writeln!(
        stderr,
        "Fatal internal error, this is a bug. Please report this to the developers"
    );
    let _ = stderr.set_color(&reset);

    default_hook(info);
}

#[derive(CliParser)]
#[clap(author, version, about)]
struct Cli {
    /// The grammar file to generate from
    grammar: PathBuf,

    /// The output directory for build artifacts
    #[clap(short, long)]
    out_dir: Option<PathBuf>,
}

struct Context {
    opts: Cli,
    stderr: StandardStream,
    /// Whether or not the last line of stderr is a progress indicator
    active_indicator: bool,
    /// The time peg-pack started
    start: Instant,
}

impl Context {
    fn new(cli: Cli) -> Self {
        // Matches clap's semantics
        let stderr_color = if atty::is(Stream::Stderr) {
            termcolor::ColorChoice::Auto
        } else {
            termcolor::ColorChoice::Never
        };

        let stderr = StandardStream::stderr(stderr_color);

        Self {
            stderr,
            opts: cli,
            active_indicator: false,
            start: Instant::now(),
        }
    }

    fn run(mut self) {
        self.set_indicator("Checking environment");
        self.check_node();
        self.check_rust();
        self.check_grammar();

        self.set_indicator("Setting up output");
        self.create_out_dir();
        self.populate_out_dir();

        self.clear_indicator();
        self.execute_grammar();

        self.set_indicator("Generating parser");
        let parser = self.load_parser();
        self.generate_code(parser);

        self.set_indicator("Compiling");
        self.compile();

        self.print_ready();
        self.execute();
    }

    fn print_ready(&mut self) {
        self.println(format!("Parser built in {:.1?}", self.start.elapsed()));
    }

    /// Load the generated IR file into a parser
    fn load_parser(&mut self) -> Parser {
        let ir = match fs::read(self.ir_file()) {
            Ok(ir) => ir,
            Err(err) => {
                self.exit_with_error(format!("Could not read IR: {}", err));
            }
        };

        match Parser::load(&ir) {
            Ok(parser) => parser,
            Err(Error::Load(message)) => self.exit_with_error(message),
            Err(Error::LeftRecursive(left_recursive)) => {
                self.print_error_heading();

                if left_recursive.len() == 1 {
                    self.print("Ill-formed grammar, ");
                    self.print_color(Color::Yellow, false);
                    self.print(left_recursive.iter().next().unwrap());
                    self.print_reset();
                    self.println(" is left-recursive");
                } else {
                    self.print("Ill-formed grammar, the following rules are left-recursive: ");

                    for (i, rule) in left_recursive.iter().enumerate() {
                        if i != 0 {
                            self.print(", ");
                        }

                        self.print_color(Color::Yellow, false);
                        self.print(rule);
                        self.print_reset();
                    }

                    self.println("");
                }

                exit(1);
            }
        }
    }

    /// Generate the Rust code for the parser
    fn generate_code(&mut self, parser: Parser) {
        let code = parser.generate();

        if let Err(err) = fs::write(self.parser_file(), code) {
            self.exit_with_error(format!("Could not write generated code: {}", err));
        }
    }

    /// Compile the parser into an executable
    fn compile(&mut self) {
        let result = Command::new("rustc")
            .args(["--edition", "2021"])
            .args(["-C", "opt-level=3"])
            .args(["-C", "target-cpu=native"])
            .arg("-o")
            .arg(self.executable_file())
            .arg(self.harness_file())
            .output();

        let result = match result {
            Ok(result) => result,
            Err(err) => {
                self.exit_with_error(format!("Could not compile parser: {}", err));
            }
        };

        if !result.status.success() {
            self.exit_with_error_and_output("Could not compile parser", &result);
        }
    }

    /// Run the parser executable
    fn execute(&mut self) {
        let result = Command::new(self.executable_file()).status();

        let status = match result {
            Ok(result) => result,
            Err(err) => {
                self.exit_with_error(format!("Could not launch parser: {}", err));
            }
        };

        if !status.success() {
            if let Some(status) = status.code() {
                self.exit_with_error(format!("Parser exited with status {}", status));
            } else {
                self.exit_with_error("Parser exited with unknown status");
            }
        }
    }

    /// Execute the grammar script and generator IR
    fn execute_grammar(&mut self) {
        if let Err(err) = self.execute_grammar_unhandled() {
            self.exit_with_error(format!("Could not run grammar script: {}", err));
        }
    }

    fn execute_grammar_unhandled(&mut self) -> io::Result<()> {
        let grammar_path = self.opts.grammar.canonicalize()?;
        let loader_path = self.loader_file();
        let ir_path = self.ir_file();

        let status = Command::new("node")
            .env("PEG_PACK_GRAMMAR", grammar_path)
            .env("PEG_PACK_IR", ir_path)
            .arg(loader_path)
            .status()?;

        if !status.success() {
            if let Some(status) = status.code() {
                self.exit_with_error(format!("Grammar script exited with status {}", status));
            } else {
                self.exit_with_error("Grammar script exited with unknown status");
            }
        }

        Ok(())
    }

    /// Check that the grammar script is an accessible file
    fn check_grammar(&mut self) {
        let grammar = &self.opts.grammar;
        let display = grammar.display();

        if let Err(err) = File::open(grammar) {
            if err.kind() == ErrorKind::NotFound {
                self.exit_with_error(format!("Grammar file does not exist ({})", display));
            } else if err.kind() == ErrorKind::PermissionDenied {
                self.exit_with_error(format!(
                    "Insufficient permissions to access grammar file ({})",
                    display
                ));
            } else {
                self.exit_with_error(format!(
                    "Could not open grammar file ({}): {}",
                    display, err
                ));
            }
        }

        if !grammar.is_file() {
            self.exit_with_error(format!("Grammar was not a file ({})", display));
        }
    }

    /// Remove the old output directory and create a new one
    fn create_out_dir(&mut self) {
        let out_dir = self.out_dir();
        let display = out_dir.display();

        if out_dir.exists() && !out_dir.is_dir() {
            self.exit_with_error(format!("Output directory is not a directory ({})", display));
        }

        if out_dir.exists() {
            if let Err(err) = fs::remove_dir_all(out_dir) {
                self.exit_with_error(format!("Could not remove old output directory: {}", err));
            }
        }

        if let Err(err) = fs::create_dir(out_dir) {
            if err.kind() == ErrorKind::NotFound {
                self.exit_with_error(format!(
                    "Parent of output directory does not exist ({})",
                    display
                ));
            } else if err.kind() == ErrorKind::PermissionDenied {
                self.exit_with_error(format!(
                    "Insufficient permissions to create output directory ({})",
                    display
                ));
            } else {
                self.exit_with_error(format!(
                    "Could not create output directory ({}): {}",
                    display, err
                ));
            }
        }
    }

    /// Populate the output directory with the required build files
    fn populate_out_dir(&mut self) {
        if let Err(err) = self.populate_out_dir_unhandled() {
            self.exit_with_error(format!("Error populating output directory: {}", err));
        }
    }

    fn populate_out_dir_unhandled(&mut self) -> io::Result<()> {
        let out_dir = self.out_dir();

        for (name, data) in OUT_DIR_FILES {
            let path = out_dir.join(name);
            assert!(path.starts_with(out_dir));

            let parent = path.parent().unwrap();
            fs::create_dir_all(parent)?;

            fs::write(path, data)?;
        }

        Ok(())
    }

    /// Check that a recent version of NodeJS is installed
    fn check_node(&mut self) {
        let command = Command::new("node").arg("--version").output();
        let version_regex = Regex::new(r"^v(\d+)\.").unwrap();

        self.check_command_installation(
            command,
            "NodeJS",
            "https://nodejs.org",
            version_regex,
            16,
            ">=16.0.0",
        );
    }

    /// Check that a recent version of Rust is installed
    fn check_rust(&mut self) {
        let command = Command::new("rustc")
            .args(["+stable", "--version"])
            .output();
        let version_regex = Regex::new(r"^rustc 1\.(\d+)\.").unwrap();

        self.check_command_installation(
            command,
            "Rust",
            "https://rustup.rs",
            version_regex,
            61,
            "^1.61.0",
        );
    }

    /// Run version command and use a regex to check its output
    fn check_command_installation(
        &mut self,
        result: io::Result<Output>,
        name: &str,
        download_url: &str,
        version_regex: Regex,
        expected_version: u32,
        expected_version_spec: &str,
    ) {
        let result = match result {
            Ok(result) => result,
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    self.exit_with_error(format!(
                        "{} was not found. Check that it is installed and added to PATH ({})",
                        name, download_url
                    ));
                } else if err.kind() == ErrorKind::PermissionDenied {
                    self.exit_with_error(format!(
                        "Insufficient permission to run {}. Reconfigure your installation, or run with more permissions",
                        name
                    ));
                } else {
                    self.exit_with_error(format!("Could not run {}: {}", name, err));
                }
            }
        };

        if !result.status.success() {
            self.exit_with_error_and_output(
                format!("{} version check returned non-zero exit status", name),
                &result,
            );
        }

        let version = version_regex.captures(&result.stdout).and_then(|captures| {
            let version_match = captures.get(1).unwrap();
            let version_str = String::from_utf8_lossy(version_match.as_bytes());
            version_str.parse::<u32>().ok()
        });

        match version {
            Some(version) if version < expected_version => {
                self.print_warn(format!(
                    "{} version ({}) out of date, expected {}",
                    name, version, expected_version_spec
                ));
            }
            None => {
                let version = String::from_utf8_lossy(&result.stdout);
                self.print_warn(format!(
                    "Could not parse {} version ({})",
                    name,
                    version.trim()
                ));
            }
            Some(_) => {}
        }
    }

    fn executable_file(&self) -> PathBuf {
        if cfg!(windows) {
            self.out_dir().join("build/parser.exe")
        } else {
            self.out_dir().join("build/parser")
        }
    }

    fn parser_file(&self) -> PathBuf {
        self.out_dir().join("parser.rs")
    }

    fn harness_file(&self) -> PathBuf {
        self.out_dir().join("build/harness.rs")
    }

    fn loader_file(&self) -> PathBuf {
        self.out_dir().join("build/loader.js")
    }

    fn ir_file(&self) -> PathBuf {
        self.out_dir().join("build/ir.json")
    }

    fn out_dir(&self) -> &Path {
        self.opts
            .out_dir
            .as_ref()
            .map(|buf| buf as &Path)
            .unwrap_or_else(|| Path::new("peg-pack-out"))
    }

    /// Prints a progress indicator, replacing the previous if possible
    fn set_indicator(&mut self, indicator: impl AsRef<str>) {
        self.clear_indicator();

        let mut color = ColorSpec::new();
        color.set_italic(true);
        color.set_fg(Some(Color::Blue));

        let mut reset = ColorSpec::new();
        reset.set_reset(true);

        let _ = self.stderr.set_color(&color);
        let _ = writeln!(self.stderr, "{}", indicator.as_ref());
        let _ = self.stderr.set_color(&reset);

        self.active_indicator = true;
    }

    /// Remove the last progress indicator if possible
    fn clear_indicator(&mut self) {
        if self.active_indicator && self.stderr.supports_color() {
            let _ = write!(self.stderr, "\x1b[F\x1b[K");
        }

        self.active_indicator = false;
    }

    /// Prints the output of a command
    fn print_output(&mut self, output: &Output) {
        self.print_output_stream("stdout", &output.stdout);
        self.print_output_stream("stderr", &output.stderr);
    }

    fn print_output_stream(&mut self, name: &str, stream: &[u8]) {
        if stream.is_empty() {
            return;
        }

        let content = String::from_utf8_lossy(stream);

        for line in content.lines() {
            self.print_color(Color::Cyan, true);
            self.print(name);
            self.print(": ");
            self.print_reset();
            self.println(line);
        }
    }

    /// Print an error message and exit
    fn exit_with_error(&mut self, message: impl AsRef<str>) -> ! {
        self.print_error(message);
        exit(1);
    }

    /// Print an error message and command output, then exit
    fn exit_with_error_and_output(&mut self, message: impl AsRef<str>, output: &Output) -> ! {
        self.print_error(message);
        self.print_output(output);
        exit(1);
    }

    fn print_error(&mut self, message: impl AsRef<str>) {
        self.print_error_heading();
        self.println(message);
    }

    fn print_warn(&mut self, message: impl AsRef<str>) {
        self.print_warn_heading();
        self.println(message);
    }

    fn print_error_heading(&mut self) {
        self.print_color(Color::Red, true);
        self.print("error: ");
        self.print_reset();
    }

    fn print_warn_heading(&mut self) {
        self.print_color(Color::Yellow, true);
        self.print("warn: ");
        self.print_reset();
    }

    fn print(&mut self, message: impl AsRef<str>) {
        self.clear_indicator();
        let _ = write!(self.stderr, "{}", message.as_ref());
    }

    fn println(&mut self, message: impl AsRef<str>) {
        self.clear_indicator();
        let _ = writeln!(self.stderr, "{}", message.as_ref());
    }

    fn print_color(&mut self, color: Color, bold: bool) {
        self.clear_indicator();
        let mut color_spec = ColorSpec::new();
        color_spec.set_fg(Some(color));
        color_spec.set_bold(bold);
        let _ = self.stderr.set_color(&color_spec);
    }

    fn print_reset(&mut self) {
        self.clear_indicator();
        let mut reset_color = ColorSpec::new();
        reset_color.set_reset(true);
        let _ = self.stderr.set_color(&reset_color);
    }
}
