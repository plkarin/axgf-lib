// SPDX-License-Identifier: Apache-2.0
//! `axgf` — command-line entry point for the reference library.
//!
//! Every subcommand is a thin shell around one public function on the
//! library's stateless JSON boundary. The uniform [`Envelope`] is printed
//! verbatim to stdout so downstream tools can pipe it through `jq`; the
//! process exit code reflects the envelope's `status`, or the count of
//! error-severity diagnostics for `validate`.
//!
//! Bundle inputs may be provided as a file path or as `-` (stdin), so any
//! command can be chained without touching the filesystem:
//!
//! ```text
//! axgf create --family-name "Karin" \
//!   | jq -c '.data' \
//!   | axgf add --input - --kind person --entity person.json
//! ```

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use axgf_rs::boundary::envelope::{DiagnosticCode, Envelope, Severity, Status};
use axgf_rs::{
    add_entity, create_bundle, deduplicate, delete_entity, export_bundle, import_bundle, inspect,
    update_entity, validate, DeletePolicy, EntityKind,
};
use base64::Engine as _;
use clap::{Parser, Subcommand, ValueEnum};

/// The AXGF command-line interface.
#[derive(Parser)]
#[command(
    name = "axgf",
    version,
    about = "Command-line interface for the Axiom Genealogy Format reference library",
    long_about = "One subcommand per V1 API function on the axgf-rs boundary. \
                  Each prints a JSON envelope on stdout; exit code is non-zero \
                  when the operation was refused (or when `validate` reports \
                  error-severity diagnostics)."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create an empty bundle stamped with the current spec version.
    Create {
        /// Populate `manifest.family.name`.
        #[arg(long, value_name = "NAME")]
        family_name: Option<String>,
    },

    /// Decode a `.axgf` ZIP archive into a flat-bundle envelope.
    Import {
        /// Path to the `.axgf` archive; `-` reads bytes from stdin.
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
    },

    /// Rebuild a `.axgf` ZIP archive from a flat bundle.
    Export {
        /// Path to the flat-bundle JSON file; `-` reads from stdin.
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        /// If set, decode the returned base64 and write the ZIP bytes to
        /// this path. The envelope is still printed on stdout.
        #[arg(long, value_name = "PATH")]
        output: Option<PathBuf>,
    },

    /// Return the manifest as-was plus freshly computed stats.
    Inspect {
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
    },

    /// Run structural + semantic validation over a flat bundle.
    ///
    /// Exits with code 2 when any error-severity diagnostic is present,
    /// even if the operation itself succeeded (this is the point of the
    /// non-blocking validation model: the report is the answer).
    Validate {
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
    },

    /// Add an entity of the given kind to a flat bundle.
    Add {
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        #[arg(long, value_enum)]
        kind: CliEntityKind,
        /// Path to the entity JSON; `-` reads from stdin.
        #[arg(long, value_name = "PATH")]
        entity: PathBuf,
    },

    /// Replace an existing entity (full replace; `entity.id` required).
    Update {
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        #[arg(long, value_enum)]
        kind: CliEntityKind,
        #[arg(long, value_name = "PATH")]
        entity: PathBuf,
    },

    /// Delete an entity by id under the given referential-integrity policy.
    Delete {
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        #[arg(long, value_enum)]
        kind: CliEntityKind,
        #[arg(long, value_name = "UUID")]
        id: String,
        #[arg(long, value_enum, default_value_t = CliPolicy::Reject)]
        policy: CliPolicy,
    },

    /// Run the safe deduplication passes.
    Dedup {
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
    },

    /// Convert a GEDCOM 5.5.1 byte stream to a flat AXGF bundle.
    #[cfg(feature = "gedcom")]
    ConvertGedcom {
        /// Path to the `.ged` file; `-` reads bytes from stdin.
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        /// Confidence assigned to imported facts when the source has none.
        #[arg(long, default_value_t = 0.8)]
        confidence: f64,
        /// BCP 47 language tag stored on imported `Place` names.
        #[arg(long, default_value = "en")]
        place_lang: String,
    },
}

// EntityKind and DeletePolicy do not derive clap's ValueEnum in the library
// (which stays clap-free), so we mirror them here and convert at the edge.

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

fn read_bytes(path: &Path) -> io::Result<Vec<u8>> {
    if path.as_os_str() == "-" {
        let mut buf = Vec::new();
        io::stdin().lock().read_to_end(&mut buf)?;
        Ok(buf)
    } else {
        std::fs::read(path)
    }
}

fn read_text(path: &Path) -> io::Result<String> {
    let bytes = read_bytes(path)?;
    String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn io_error_envelope(context: &str, err: impl std::fmt::Display) -> Envelope {
    // I/O and decoding failures are the CLI's own concern, not the library's;
    // `INTERNAL` is the closest fit in the stable diagnostic vocabulary.
    Envelope::error(DiagnosticCode::Internal, format!("{context}: {err}"))
}

/// Turn a subcommand into the resulting `Envelope`. Every I/O failure is
/// folded into an error envelope so the caller can print it uniformly.
fn dispatch(cmd: Command) -> Envelope {
    match cmd {
        Command::Create { family_name } => create_bundle(family_name.as_deref()),

        Command::Import { input } => match read_bytes(&input) {
            Ok(bytes) => import_bundle(&bytes),
            Err(e) => io_error_envelope(&format!("reading {}", input.display()), e),
        },

        Command::Export { input, output } => {
            let flat = match read_text(&input) {
                Ok(s) => s,
                Err(e) => return io_error_envelope(&format!("reading {}", input.display()), e),
            };
            let env = export_bundle(&flat);
            if let Some(out_path) = output.as_ref() {
                if let Some(b64) = env.data.get("zip_base64").and_then(|v| v.as_str()) {
                    match base64::engine::general_purpose::STANDARD.decode(b64) {
                        Ok(bytes) => {
                            if let Err(e) = std::fs::write(out_path, bytes) {
                                return io_error_envelope(
                                    &format!("writing {}", out_path.display()),
                                    e,
                                );
                            }
                        }
                        Err(e) => {
                            return io_error_envelope("decoding zip_base64", e);
                        }
                    }
                }
                // If the export itself failed, `data` is null — surface the
                // library's own error envelope untouched. No file is written.
            }
            env
        }

        Command::Inspect { input } => match read_text(&input) {
            Ok(flat) => inspect(&flat),
            Err(e) => io_error_envelope(&format!("reading {}", input.display()), e),
        },

        Command::Validate { input } => match read_text(&input) {
            Ok(flat) => validate(&flat),
            Err(e) => io_error_envelope(&format!("reading {}", input.display()), e),
        },

        Command::Add {
            input,
            kind,
            entity,
        } => {
            let flat = match read_text(&input) {
                Ok(s) => s,
                Err(e) => return io_error_envelope(&format!("reading {}", input.display()), e),
            };
            let entity_json = match read_text(&entity) {
                Ok(s) => s,
                Err(e) => return io_error_envelope(&format!("reading {}", entity.display()), e),
            };
            add_entity(&flat, kind.into(), &entity_json)
        }

        Command::Update {
            input,
            kind,
            entity,
        } => {
            let flat = match read_text(&input) {
                Ok(s) => s,
                Err(e) => return io_error_envelope(&format!("reading {}", input.display()), e),
            };
            let entity_json = match read_text(&entity) {
                Ok(s) => s,
                Err(e) => return io_error_envelope(&format!("reading {}", entity.display()), e),
            };
            update_entity(&flat, kind.into(), &entity_json)
        }

        Command::Delete {
            input,
            kind,
            id,
            policy,
        } => {
            let flat = match read_text(&input) {
                Ok(s) => s,
                Err(e) => return io_error_envelope(&format!("reading {}", input.display()), e),
            };
            delete_entity(&flat, kind.into(), &id, policy.into())
        }

        Command::Dedup { input } => match read_text(&input) {
            Ok(flat) => deduplicate(&flat),
            Err(e) => io_error_envelope(&format!("reading {}", input.display()), e),
        },

        #[cfg(feature = "gedcom")]
        Command::ConvertGedcom {
            input,
            confidence,
            place_lang,
        } => match read_bytes(&input) {
            Ok(bytes) => axgf_rs::convert_gedcom(&bytes, confidence, &place_lang),
            Err(e) => io_error_envelope(&format!("reading {}", input.display()), e),
        },
    }
}

fn exit_code(cmd_was_validate: bool, env: &Envelope) -> ExitCode {
    if env.status == Status::Error {
        return ExitCode::from(1);
    }
    if cmd_was_validate
        && env
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    {
        // Validate is a report, not an action; the library returns Ok even on
        // hard structural problems. The CLI escalates to a non-zero code so
        // CI pipelines and shell one-liners can gate on it.
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let is_validate = matches!(cli.command, Command::Validate { .. });
    let env = dispatch(cli.command);
    // to_json is infallible; a broken stdout (piped to `head`) is not fatal.
    let _ = writeln!(io::stdout(), "{}", env.to_json());
    exit_code(is_validate, &env)
}
