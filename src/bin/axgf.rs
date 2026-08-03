// SPDX-License-Identifier: Apache-2.0
//! `axgf` — command-line entry point for the reference library.
//!
//! Every subcommand is a thin shell around one public function on the
//! library's stateless JSON boundary. The default output is a concise
//! human summary on stdout; `--json` selects the raw JSON envelope for
//! piping into `jq`, and `-q/--quiet` suppresses stdout entirely and
//! carries the result in the exit code.
//!
//! Bundle inputs can be passed as a positional argument (preferred) or
//! via `--input` (kept as an unreleased-back-compat alias). Read-only
//! commands (`inspect`, `validate`, `import`) never take `--output`;
//! mutating commands (`create`, `convert-gedcom`, `add`, `update`,
//! `delete`, `dedup`, `export`) take `-o/--output`, and if omitted on a
//! command that read a file, edit that file in place — but only after
//! the new bytes are ready.

use std::collections::BTreeMap;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use axgf_rs::boundary::envelope::{DiagnosticCode, Envelope, Severity, Status};
use axgf_rs::{
    add_entity, create_bundle, deduplicate, delete_entity, export_bundle, import_bundle, inspect,
    update_entity, validate, DeletePolicy, EntityKind,
};
use base64::Engine as _;
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde_json::Value;

/// The AXGF command-line interface.
#[derive(Parser)]
#[command(
    name = "axgf",
    version,
    about = "Command-line interface for the Axiom Genealogy Format reference library",
    long_about = "One subcommand per V1 API function on the axgf-rs boundary. \
                  Default output is a concise human summary; `--json` selects \
                  the raw envelope (pipeable through `jq`); `-q/--quiet` \
                  carries the outcome in the exit code."
)]
struct Cli {
    /// Print the raw JSON envelope on stdout and nothing else.
    #[arg(long, global = true)]
    json: bool,
    /// Suppress stdout entirely; the exit code carries the result.
    #[arg(short, long, global = true)]
    quiet: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create an empty bundle stamped with the current spec version.
    Create {
        /// Populate `manifest.family.name`.
        #[arg(long, value_name = "NAME", alias = "family-name")]
        name: Option<String>,
        /// Where to write the created bundle (`.axgf` or `.json`).
        #[arg(short, long, value_name = "PATH")]
        output: Option<PathBuf>,
    },

    /// Decode a `.axgf` archive into a flat-bundle envelope (read-only).
    Import {
        #[command(flatten)]
        input: InputPath,
    },

    /// Rebuild a bundle from a flat bundle.
    Export {
        #[command(flatten)]
        input: InputPath,
        /// Where to write the rebuilt bundle (`.axgf` or `.json`).
        #[arg(short, long, value_name = "PATH")]
        output: Option<PathBuf>,
    },

    /// Print manifest and freshly computed stats (read-only).
    Inspect {
        #[command(flatten)]
        input: InputPath,
    },

    /// Structural + semantic report (read-only). Exits 2 on error-severity
    /// diagnostics so CI pipelines can gate on the outcome.
    Validate {
        #[command(flatten)]
        input: InputPath,
    },

    /// Add an entity of the given kind.
    Add {
        /// Entity kind: person, family, event, link, occupation, source,
        /// place, or document.
        #[arg(value_enum)]
        kind: CliEntityKind,
        #[command(flatten)]
        input: InputPath,
        /// Path to the entity JSON (`-` reads from stdin).
        #[arg(long, value_name = "PATH", alias = "entity")]
        data: PathBuf,
        /// Where to write the resulting bundle (defaults to editing the
        /// input file in place).
        #[arg(short, long, value_name = "PATH")]
        output: Option<PathBuf>,
    },

    /// Replace an existing entity (full replace; `entity.id` required).
    Update {
        #[arg(value_enum)]
        kind: CliEntityKind,
        #[command(flatten)]
        input: InputPath,
        #[arg(long, value_name = "PATH", alias = "entity")]
        data: PathBuf,
        #[arg(short, long, value_name = "PATH")]
        output: Option<PathBuf>,
    },

    /// Delete an entity by id under the given referential-integrity policy.
    Delete {
        #[arg(value_enum)]
        kind: CliEntityKind,
        #[command(flatten)]
        input: InputPath,
        #[arg(long, value_name = "UUID")]
        id: String,
        #[arg(long, value_enum, default_value_t = CliPolicy::Reject)]
        policy: CliPolicy,
        #[arg(short, long, value_name = "PATH")]
        output: Option<PathBuf>,
    },

    /// Run the safe deduplication passes.
    Dedup {
        #[command(flatten)]
        input: InputPath,
        #[arg(short, long, value_name = "PATH")]
        output: Option<PathBuf>,
    },

    /// Convert a GEDCOM 5.5.1 byte stream to an AXGF bundle.
    #[cfg(feature = "gedcom")]
    ConvertGedcom {
        #[command(flatten)]
        input: InputPath,
        /// Where to write the resulting bundle (`.axgf` or `.json`).
        #[arg(short, long, value_name = "PATH")]
        output: Option<PathBuf>,
        /// Confidence assigned to imported facts when the source has none.
        #[arg(long, default_value_t = 0.8)]
        confidence: f64,
        /// BCP 47 language tag stored on imported `Place` names.
        #[arg(long, default_value = "en")]
        place_lang: String,
    },
}

/// Input path: preferably positional, `--input` accepted as an alias.
#[derive(Args, Debug)]
#[group(required = true, multiple = false)]
struct InputPath {
    /// Path to the input file (`-` reads bytes from stdin).
    #[arg(value_name = "PATH")]
    positional: Option<PathBuf>,
    /// Backward-compatible alias for the positional argument.
    #[arg(long = "input", value_name = "PATH")]
    flag: Option<PathBuf>,
}

impl InputPath {
    fn path(&self) -> &Path {
        // clap's group=required guarantees at least one is set.
        self.positional
            .as_deref()
            .or(self.flag.as_deref())
            .expect("clap group ensures one is set")
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
#[value(rename_all = "lowercase")]
enum CliEntityKind {
    Person,
    Family,
    Event,
    Link,
    Occupation,
    Source,
    Place,
    Document,
}

impl From<CliEntityKind> for EntityKind {
    fn from(k: CliEntityKind) -> Self {
        match k {
            CliEntityKind::Person => Self::Person,
            CliEntityKind::Family => Self::Family,
            CliEntityKind::Event => Self::Event,
            CliEntityKind::Link => Self::Link,
            CliEntityKind::Occupation => Self::Occupation,
            CliEntityKind::Source => Self::Source,
            CliEntityKind::Place => Self::Place,
            CliEntityKind::Document => Self::Document,
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
#[value(rename_all = "lowercase")]
enum CliPolicy {
    Reject,
    Cascade,
    Orphan,
}

impl From<CliPolicy> for DeletePolicy {
    fn from(p: CliPolicy) -> Self {
        match p {
            CliPolicy::Reject => Self::Reject,
            CliPolicy::Cascade => Self::Cascade,
            CliPolicy::Orphan => Self::Orphan,
        }
    }
}

// =========================================================================
// I/O helpers — reading bytes and reading a flat bundle from a path
// =========================================================================

fn read_bytes(path: &Path) -> io::Result<Vec<u8>> {
    if path.as_os_str() == "-" {
        let mut buf = Vec::new();
        io::stdin().lock().read_to_end(&mut buf)?;
        Ok(buf)
    } else {
        std::fs::read(path)
    }
}

fn is_axgf_path(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("axgf"))
        .unwrap_or(false)
}

fn io_error_envelope(context: &str, err: impl std::fmt::Display) -> Envelope {
    Envelope::error(DiagnosticCode::Internal, format!("{context}: {err}"))
}

/// Read a bundle from a path, transparently importing `.axgf` archives
/// into their flat form. Stdin (`-`) is always treated as flat JSON.
fn read_flat_bundle(path: &Path) -> Result<String, Envelope> {
    if is_axgf_path(path) {
        let bytes = read_bytes(path)
            .map_err(|e| io_error_envelope(&format!("reading {}", path.display()), e))?;
        let env = import_bundle(&bytes);
        if env.status == Status::Error {
            return Err(env);
        }
        Ok(env.data.to_string())
    } else {
        let bytes = read_bytes(path)
            .map_err(|e| io_error_envelope(&format!("reading {}", path.display()), e))?;
        String::from_utf8(bytes)
            .map_err(|e| io_error_envelope(&format!("reading {}", path.display()), e))
    }
}

/// Write a flat bundle to `path`, choosing the on-disk encoding by
/// extension: `.axgf` → ZIP (via `export_bundle`), anything else → flat
/// JSON. Returns the written byte count on success.
fn write_bundle(path: &Path, flat: &Value) -> Result<usize, Envelope> {
    if is_axgf_path(path) {
        let flat_text = flat.to_string();
        let env = export_bundle(&flat_text);
        if env.status == Status::Error {
            return Err(env);
        }
        let b64 = env
            .data
            .get("zip_base64")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                Envelope::error(
                    DiagnosticCode::Internal,
                    "export_bundle envelope did not contain zip_base64",
                )
            })?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| io_error_envelope("decoding zip_base64", e))?;
        // Write to a sibling temp file then rename, so an in-place edit
        // never truncates the input before the new bytes are ready.
        write_atomic(path, &bytes).map(|_| bytes.len())
    } else {
        let text = serde_json::to_string(flat)
            .map_err(|e| io_error_envelope("serializing flat bundle", e))?;
        write_atomic(path, text.as_bytes()).map(|_| text.len())
    }
}

/// Write to a temp sibling and rename over the target so the original
/// file is never truncated on failure. If `path` has no parent (bare
/// filename in cwd) we still get atomic semantics via the same-dir temp.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), Envelope> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("axgf-tmp");
    let tmp_name = format!(".{file_name}.tmp.{}", std::process::id());
    let tmp_path = match parent {
        Some(p) => p.join(&tmp_name),
        None => PathBuf::from(&tmp_name),
    };
    std::fs::write(&tmp_path, bytes)
        .map_err(|e| io_error_envelope(&format!("writing {}", tmp_path.display()), e))?;
    std::fs::rename(&tmp_path, path).map_err(|e| {
        // Best-effort cleanup so we don't leave the temp file behind.
        let _ = std::fs::remove_file(&tmp_path);
        io_error_envelope(&format!("renaming to {}", path.display()), e)
    })
}

// =========================================================================
// Output modes: --json / default (human) / --quiet
// =========================================================================

#[derive(Copy, Clone, Debug)]
enum OutputMode {
    Json,
    Human,
    Quiet,
}

impl OutputMode {
    fn resolve(cli: &Cli) -> Self {
        if cli.quiet {
            Self::Quiet
        } else if cli.json {
            Self::Json
        } else {
            Self::Human
        }
    }
}

// =========================================================================
// Command execution
//
// Each command is executed in three steps: read input → run library →
// write output & print. The result of the whole run is a `RunResult`,
// which the top-level uses to pick an exit code.
// =========================================================================

struct RunResult {
    envelope: Envelope,
    /// Set on validate() so the top-level can escalate to exit code 2.
    validate_report: bool,
}

impl RunResult {
    fn from(env: Envelope) -> Self {
        Self {
            envelope: env,
            validate_report: false,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let mode = OutputMode::resolve(&cli);
    let result = execute(cli.command, mode);
    let exit = pick_exit(&result);

    // Emit the requested output form. Errors are always reported on
    // stderr in human mode; the envelope goes to stdout only under
    // `--json`.
    match mode {
        OutputMode::Json => {
            let _ = writeln!(io::stdout(), "{}", result.envelope.to_json());
        }
        OutputMode::Quiet => {}
        OutputMode::Human => {
            // The RunResult carries the human-form summary already
            // written to stdout by `execute`; only the error path needs
            // cleaning up here.
            if result.envelope.status == Status::Error {
                emit_errors_stderr(&result.envelope);
            }
        }
    }
    exit
}

fn pick_exit(r: &RunResult) -> ExitCode {
    if r.envelope.status == Status::Error {
        return ExitCode::from(1);
    }
    if r.validate_report
        && r.envelope
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    {
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}

fn emit_errors_stderr(env: &Envelope) {
    for d in &env.diagnostics {
        let _ = writeln!(io::stderr(), "{}: {}", d.code.as_str(), d.message);
    }
}

/// Dispatch a subcommand and produce a `RunResult`. Human-mode summary
/// prints are performed here (side-effecting stdout/stderr) so the
/// operation's context — input path, output path, etc. — is available.
fn execute(cmd: Command, mode: OutputMode) -> RunResult {
    match cmd {
        Command::Create { name, output } => cmd_create(name, output, mode),
        Command::Import { input } => cmd_import(input, mode),
        Command::Export { input, output } => cmd_export(input, output, mode),
        Command::Inspect { input } => cmd_inspect(input, mode),
        Command::Validate { input } => cmd_validate(input, mode),
        Command::Add {
            kind,
            input,
            data,
            output,
        } => cmd_mutate("add", kind, input, output, mode, |flat| {
            let entity = match read_text_bytes(&data) {
                Ok(s) => s,
                Err(e) => return e,
            };
            add_entity(&flat, kind.into(), &entity)
        }),
        Command::Update {
            kind,
            input,
            data,
            output,
        } => cmd_mutate("updated", kind, input, output, mode, |flat| {
            let entity = match read_text_bytes(&data) {
                Ok(s) => s,
                Err(e) => return e,
            };
            update_entity(&flat, kind.into(), &entity)
        }),
        Command::Delete {
            kind,
            input,
            id,
            policy,
            output,
        } => cmd_mutate("deleted", kind, input, output, mode, |flat| {
            delete_entity(&flat, kind.into(), &id, policy.into())
        }),
        Command::Dedup { input, output } => cmd_dedup(input, output, mode),
        #[cfg(feature = "gedcom")]
        Command::ConvertGedcom {
            input,
            output,
            confidence,
            place_lang,
        } => cmd_convert_gedcom(input, output, confidence, place_lang, mode),
    }
}

fn read_text_bytes(path: &Path) -> Result<String, Envelope> {
    let bytes = read_bytes(path)
        .map_err(|e| io_error_envelope(&format!("reading {}", path.display()), e))?;
    String::from_utf8(bytes)
        .map_err(|e| io_error_envelope(&format!("reading {}", path.display()), e))
}

// -------------------------------------------------------------------------
// create
// -------------------------------------------------------------------------

fn cmd_create(name: Option<String>, output: Option<PathBuf>, mode: OutputMode) -> RunResult {
    let env = create_bundle(name.as_deref());
    // If `--json` is set, writing a file is optional; otherwise -o is
    // the whole point of `create` (there is no input to edit in place).
    if output.is_none() && !matches!(mode, OutputMode::Json | OutputMode::Quiet) {
        return RunResult::from(Envelope::error(
            DiagnosticCode::Internal,
            "create requires -o/--output (or --json to print the envelope)",
        ));
    }
    if env.status == Status::Error {
        return RunResult::from(env);
    }
    if let Some(out) = output.as_ref() {
        let bytes_written = match write_bundle(out, &env.data) {
            Ok(n) => n,
            Err(e) => return RunResult::from(e),
        };
        if matches!(mode, OutputMode::Human) {
            print_key_value_table("created bundle", key_value_manifest(&env.data));
            print_wrote(out, bytes_written);
        }
    } else if matches!(mode, OutputMode::Human) {
        print_key_value_table("created bundle", key_value_manifest(&env.data));
    }
    RunResult::from(env)
}

// -------------------------------------------------------------------------
// import (read-only): print flat-bundle summary
// -------------------------------------------------------------------------

fn cmd_import(input: InputPath, mode: OutputMode) -> RunResult {
    let path = input.path();
    let bytes = match read_bytes(path) {
        Ok(b) => b,
        Err(e) => {
            return RunResult::from(io_error_envelope(&format!("reading {}", path.display()), e))
        }
    };
    let env = import_bundle(&bytes);
    if matches!(mode, OutputMode::Human) && env.status == Status::Ok {
        print_key_value_table(
            &format!("imported {}", display_short(path)),
            key_value_manifest(&env.data),
        );
        emit_grouped_diagnostics_stderr(&env);
    }
    RunResult::from(env)
}

// -------------------------------------------------------------------------
// export (mutating: writes a bundle file, defaults to unchanged input)
// -------------------------------------------------------------------------

fn cmd_export(input: InputPath, output: Option<PathBuf>, mode: OutputMode) -> RunResult {
    let path = input.path();
    let flat = match read_flat_bundle(path) {
        Ok(s) => s,
        Err(e) => return RunResult::from(e),
    };
    let env = export_bundle(&flat);
    if env.status == Status::Error {
        return RunResult::from(env);
    }

    // `export` explicitly needs a destination; if the user didn't give
    // one and did not ask for --json, we cannot know what they want.
    if output.is_none() && !matches!(mode, OutputMode::Json | OutputMode::Quiet) {
        return RunResult::from(Envelope::error(
            DiagnosticCode::Internal,
            "export requires -o/--output (or --json to receive base64 in the envelope)",
        ));
    }

    if let Some(out) = output.as_ref() {
        // Decode once, write to disk. If out is .json, we write the
        // flat form instead of the ZIP.
        let bytes_written = if is_axgf_path(out) {
            let b64 = env
                .data
                .get("zip_base64")
                .and_then(Value::as_str)
                .unwrap_or("");
            let zip_bytes = match base64::engine::general_purpose::STANDARD.decode(b64) {
                Ok(b) => b,
                Err(e) => return RunResult::from(io_error_envelope("decoding zip_base64", e)),
            };
            if let Err(e) = write_atomic(out, &zip_bytes) {
                return RunResult::from(e);
            }
            zip_bytes.len()
        } else {
            // Writing flat JSON: parse-once-write-once path.
            let parsed: Value = match serde_json::from_str(&flat) {
                Ok(v) => v,
                Err(e) => {
                    return RunResult::from(io_error_envelope("re-serializing flat bundle", e))
                }
            };
            match write_bundle(out, &parsed) {
                Ok(n) => n,
                Err(e) => return RunResult::from(e),
            }
        };
        if matches!(mode, OutputMode::Human) {
            let _ = writeln!(io::stdout(), "exported {}", display_short(path));
            print_wrote(out, bytes_written);
        }
    }
    RunResult::from(env)
}

// -------------------------------------------------------------------------
// inspect / validate (read-only)
// -------------------------------------------------------------------------

fn cmd_inspect(input: InputPath, mode: OutputMode) -> RunResult {
    let path = input.path();
    let flat = match read_flat_bundle(path) {
        Ok(s) => s,
        Err(e) => return RunResult::from(e),
    };
    let env = inspect(&flat);
    if matches!(mode, OutputMode::Human) && env.status == Status::Ok {
        // For inspect the envelope's data is {manifest, stats}. Print
        // the filename as a header, then a merged manifest+stats table.
        let _ = writeln!(io::stdout(), "{}", display_short(path));
        let mut rows: Vec<(String, String)> = Vec::new();
        if let Some(v) = env.data["manifest"].get("axgf").and_then(Value::as_str) {
            rows.push(("axgf".into(), v.into()));
        }
        if let Some(v) = env.data["manifest"]["family"]
            .get("name")
            .and_then(Value::as_str)
        {
            rows.push(("family".into(), v.into()));
        }
        for (label, key) in stat_labels() {
            let n = env.data["stats"]
                .get(key)
                .and_then(Value::as_u64)
                .unwrap_or(0);
            rows.push((label.into(), n.to_string()));
        }
        print_rows(rows);
        emit_grouped_diagnostics_stderr(&env);
    }
    RunResult::from(env)
}

fn cmd_validate(input: InputPath, mode: OutputMode) -> RunResult {
    let path = input.path();
    let flat = match read_flat_bundle(path) {
        Ok(s) => s,
        Err(e) => return RunResult::from(e),
    };
    let env = validate(&flat);
    if matches!(mode, OutputMode::Human) && env.status == Status::Ok {
        let _ = writeln!(io::stdout(), "validated {}", display_short(path));
        let errors = env.data.get("errors").and_then(Value::as_u64).unwrap_or(0);
        let warnings = env
            .data
            .get("warnings")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let mut rows: Vec<(String, String)> = vec![
            ("errors".into(), errors.to_string()),
            ("warnings".into(), warnings.to_string()),
        ];
        // Group diagnostics by code and append to the summary. The
        // grouped counts are the actual answer, so they belong on
        // stdout alongside the summary rather than stderr.
        for (code, count) in group_by_code(&env.diagnostics) {
            rows.push((code, count.to_string()));
        }
        print_rows(rows);
    }
    RunResult {
        envelope: env,
        validate_report: true,
    }
}

// -------------------------------------------------------------------------
// mutating commands (add / update / delete)
// -------------------------------------------------------------------------

fn cmd_mutate<F>(
    verb: &str,
    kind: CliEntityKind,
    input: InputPath,
    output: Option<PathBuf>,
    mode: OutputMode,
    op: F,
) -> RunResult
where
    F: FnOnce(String) -> Envelope,
{
    let in_path = input.path().to_path_buf();
    let flat = match read_flat_bundle(&in_path) {
        Ok(s) => s,
        Err(e) => return RunResult::from(e),
    };
    let env = op(flat);
    if env.status == Status::Error {
        return RunResult::from(env);
    }
    let dest = output.as_deref().unwrap_or(&in_path);
    // The envelope's data is {"id": …, "bundle": <flat>}. Persist the
    // bundle, then print a one-line summary and the wrote-line.
    let bundle = env.data.get("bundle").cloned().unwrap_or(Value::Null);
    let bytes_written = match write_bundle(dest, &bundle) {
        Ok(n) => n,
        Err(e) => return RunResult::from(e),
    };
    if matches!(mode, OutputMode::Human) {
        let id = env
            .data
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("<unknown>");
        // Verbs: add prints "added <kind> <id>", update prints "updated
        // <kind> <id>", delete prints "deleted <kind> <id>".
        let action_word = match verb {
            "add" => "added",
            _ => verb,
        };
        let _ = writeln!(
            io::stdout(),
            "{action_word} {kind} {id}",
            kind = kind_singular(kind)
        );
        emit_grouped_diagnostics_stderr(&env);
        print_wrote(dest, bytes_written);
    }
    RunResult::from(env)
}

fn kind_singular(k: CliEntityKind) -> &'static str {
    EntityKind::from(k).singular()
}

// -------------------------------------------------------------------------
// dedup
// -------------------------------------------------------------------------

fn cmd_dedup(input: InputPath, output: Option<PathBuf>, mode: OutputMode) -> RunResult {
    let in_path = input.path().to_path_buf();
    let flat = match read_flat_bundle(&in_path) {
        Ok(s) => s,
        Err(e) => return RunResult::from(e),
    };
    let env = deduplicate(&flat);
    if env.status == Status::Error {
        return RunResult::from(env);
    }
    let dest = output.as_deref().unwrap_or(&in_path);
    let bundle = env.data.get("bundle").cloned().unwrap_or(Value::Null);
    let bytes_written = match write_bundle(dest, &bundle) {
        Ok(n) => n,
        Err(e) => return RunResult::from(e),
    };
    if matches!(mode, OutputMode::Human) {
        let _ = writeln!(io::stdout(), "deduplicated {}", display_short(&in_path));
        let merged_p = env
            .data
            .get("merged_persons")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let merged_f = env
            .data
            .get("merged_families")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let manual = env
            .data
            .get("manual_review")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        print_rows(vec![
            ("merged persons".into(), merged_p.to_string()),
            ("merged families".into(), merged_f.to_string()),
            ("manual review".into(), manual.to_string()),
        ]);
        emit_grouped_diagnostics_stderr(&env);
        print_wrote(dest, bytes_written);
    }
    RunResult::from(env)
}

// -------------------------------------------------------------------------
// convert-gedcom
// -------------------------------------------------------------------------

#[cfg(feature = "gedcom")]
fn cmd_convert_gedcom(
    input: InputPath,
    output: Option<PathBuf>,
    confidence: f64,
    place_lang: String,
    mode: OutputMode,
) -> RunResult {
    let path = input.path();
    let bytes = match read_bytes(path) {
        Ok(b) => b,
        Err(e) => {
            return RunResult::from(io_error_envelope(&format!("reading {}", path.display()), e))
        }
    };
    let env = axgf_rs::convert_gedcom(&bytes, confidence, &place_lang);
    if env.status == Status::Error {
        return RunResult::from(env);
    }

    // convert-gedcom has no in-place semantics — its input is .ged. If
    // the caller didn't give -o and didn't ask for --json/--quiet we
    // can't infer where to write.
    if output.is_none() && !matches!(mode, OutputMode::Json | OutputMode::Quiet) {
        return RunResult::from(Envelope::error(
            DiagnosticCode::Internal,
            "convert-gedcom requires -o/--output (or --json to print the envelope)",
        ));
    }

    if let Some(out) = output.as_ref() {
        let bundle = env.data.get("bundle").cloned().unwrap_or(Value::Null);
        let bytes_written = match write_bundle(out, &bundle) {
            Ok(n) => n,
            Err(e) => return RunResult::from(e),
        };
        if matches!(mode, OutputMode::Human) {
            let _ = writeln!(io::stdout(), "converted {}", display_short(path));
            print_key_value_table_headerless(key_value_manifest(
                &env.data.get("bundle").cloned().unwrap_or(Value::Null),
            ));
            emit_grouped_diagnostics_stderr(&env);
            print_wrote(out, bytes_written);
        }
    }
    RunResult::from(env)
}

// =========================================================================
// Human-form printing helpers
// =========================================================================

fn stat_labels() -> [(&'static str, &'static str); 8] {
    [
        ("persons", "persons"),
        ("families", "families"),
        ("events", "events"),
        ("links", "links"),
        ("occupations", "occupations"),
        ("sources", "sources"),
        ("places", "places"),
        ("documents", "documents"),
    ]
}

/// Read `manifest.axgf`, `manifest.family.name`, and the eight stat
/// counts out of a flat-bundle Value for the summary tables.
fn key_value_manifest(flat: &Value) -> Vec<(String, String)> {
    let mut rows: Vec<(String, String)> = Vec::new();
    let stats = flat.get("manifest").and_then(|m| m.get("stats"));
    for (label, key) in stat_labels() {
        let n = stats
            .and_then(|s| s.get(key))
            .and_then(Value::as_u64)
            .or_else(|| {
                flat.get(key)
                    .and_then(|v| v.as_object().map(|m| m.len() as u64))
            })
            .unwrap_or(0);
        rows.push((label.into(), n.to_string()));
    }
    rows
}

fn print_key_value_table(header: &str, rows: Vec<(String, String)>) {
    let _ = writeln!(io::stdout(), "{header}");
    print_rows(rows);
}

fn print_key_value_table_headerless(rows: Vec<(String, String)>) {
    print_rows(rows);
}

/// Format a two-column table: label left-aligned, then values. Values
/// are right-aligned when they are all digit strings (looks like the
/// spec's numeric summaries), otherwise left-aligned (mixed content
/// like `inspect`'s manifest fields).
fn print_rows(rows: Vec<(String, String)>) {
    let label_w = rows.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    let value_w = rows.iter().map(|(_, v)| v.len()).max().unwrap_or(0);
    let all_numeric = rows
        .iter()
        .all(|(_, v)| !v.is_empty() && v.chars().all(|c| c.is_ascii_digit()));
    for (k, v) in rows {
        if all_numeric {
            let _ = writeln!(
                io::stdout(),
                "  {k:<label_w$}   {v:>value_w$}",
                label_w = label_w,
                value_w = value_w
            );
        } else {
            let _ = writeln!(io::stdout(), "  {k:<label_w$}   {v}", label_w = label_w);
        }
    }
}

/// Print the "wrote family.axgf (84 KiB)" line — trailing summary for
/// commands that persisted a bundle.
fn print_wrote(path: &Path, bytes: usize) {
    let _ = writeln!(
        io::stdout(),
        "wrote {} ({})",
        display_short(path),
        format_size(bytes)
    );
}

fn display_short(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.display().to_string())
}

/// Human size: "N B", "N KiB", "N MiB" (0 or 1 decimal place).
fn format_size(n: usize) -> String {
    const KIB: usize = 1024;
    const MIB: usize = 1024 * 1024;
    if n < KIB {
        format!("{n} B")
    } else if n < MIB {
        format!("{} KiB", n / KIB)
    } else {
        let m = n as f64 / MIB as f64;
        if m >= 10.0 {
            format!("{m:.0} MiB")
        } else {
            format!("{m:.1} MiB")
        }
    }
}

/// Group the envelope's diagnostics by code, preserving first-seen
/// order. Used for the counts breakdown printed on stderr / stdout.
fn group_by_code(diags: &[axgf_rs::boundary::envelope::Diagnostic]) -> Vec<(String, usize)> {
    let mut order: Vec<String> = Vec::new();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for d in diags {
        let key = d.code.as_str().to_string();
        if !counts.contains_key(&key) {
            order.push(key.clone());
        }
        *counts.entry(key).or_insert(0) += 1;
    }
    order
        .into_iter()
        .map(|k| {
            let n = counts.get(&k).copied().unwrap_or(0);
            (k, n)
        })
        .collect()
}

/// Print grouped diagnostic counts on stderr, one line per code, sorted
/// by descending count for readability. Silently skipped when there are
/// no diagnostics.
fn emit_grouped_diagnostics_stderr(env: &Envelope) {
    if env.diagnostics.is_empty() {
        return;
    }
    let mut grouped = group_by_code(&env.diagnostics);
    grouped.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let label_w = grouped.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    for (code, count) in grouped {
        let _ = writeln!(
            io::stderr(),
            "  {code:<label_w$}   {count}",
            label_w = label_w
        );
    }
}
