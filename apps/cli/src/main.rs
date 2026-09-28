mod host;
mod project;
mod web;
mod workflow;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use couch_local_library::Library;
use couch_manifests::{Manifest, Target};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "couch",
    version,
    about = "Giga Couch local package tools (no engine required)"
)]
struct Cli {
    /// Print one JSON result envelope, including operational errors.
    #[arg(long, global = true)]
    json: bool,
    /// Override the per-user application data directory.
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
    /// Godot executable or macOS .app; overrides COUCH_GODOT and discovery.
    #[arg(long, global = true)]
    godot: Option<PathBuf>,
    /// Time allowed per Godot version probe, including first-run OS checks.
    #[arg(long, global = true, default_value_t = 15, value_parser = clap::value_parser!(u64).range(1..=120))]
    godot_timeout_secs: u64,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show configuration and probe Godot's version. Never installs tools.
    Doctor {
        /// Exit with status 1 unless a supported Godot installation is found.
        #[arg(long)]
        require_godot: bool,
        /// Inspect a Godot project without launching Godot.
        #[arg(long, value_name = "PATH")]
        project: Option<PathBuf>,
    },
    /// Copy a template into a new project folder and register it. Never installs Godot.
    Init {
        /// Game title (1–80 characters). The folder name is a lowercase slug.
        title: String,
        /// Existing directory that will contain the new project folder.
        #[arg(long)]
        parent: PathBuf,
        /// Template project to copy, or 2d-couch / 3d-couch.
        #[arg(long, value_name = "PATH")]
        template: Option<PathBuf>,
    },
    /// Static project diagnostics. Does not launch Godot.
    Check {
        #[arg(long)]
        project: PathBuf,
    },
    /// Play the project in the installed Godot editor. Does not download templates.
    Run {
        #[arg(long)]
        project: PathBuf,
    },
    /// Export a Godot PCK if export templates are already installed. Never downloads them.
    Pack {
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        target: Option<Target>,
    },
    /// Copy a packed release to a local private drop. No account upload.
    Publish {
        #[arg(long)]
        project: PathBuf,
        #[arg(long, default_value = "private")]
        visibility: String,
    },
    /// Check manifest and every declared artifact's size and SHA-256.
    Validate { manifest: PathBuf },
    /// Import unsigned local content. Does not install a runtime or run a game.
    Install {
        manifest: PathBuf,
        /// Defaults to this computer's OS/architecture.
        #[arg(long)]
        target: Option<Target>,
    },
    /// List all installed releases, including inactive previous versions.
    Library,
    /// Open the couch Home: sign in, browse the library, and play.
    WebHome {
        /// Open a window instead of fullscreen kiosk.
        #[arg(long)]
        windowed: bool,
        /// Serve Home and print its origin without starting a shell. The
        /// packaged app uses this and closes stdin when it quits.
        #[arg(long, conflicts_with = "windowed")]
        serve_only: bool,
        /// Platform API. Home still opens if this server is not running.
        #[arg(long, default_value = "http://127.0.0.1:8787")]
        platform: String,
    },
    /// Run the platform server: accounts, the master library, and downloads.
    Platform {
        /// Address for this computer. The player app does not open the database.
        #[arg(long, default_value = "127.0.0.1:8787")]
        bind: String,
    },
    /// Serve a web-1 package from a loopback origin. Does not download a browser.
    WebServe {
        #[arg(long)]
        package: PathBuf,
        /// Save directory. Defaults to the guest slot for this game.
        #[arg(long)]
        save_dir: Option<PathBuf>,
    },
    /// Open a web-1 package in the Giga Couch browser shell.
    WebRun {
        #[arg(long)]
        package: PathBuf,
        /// Save directory. Defaults to the guest slot for this game.
        #[arg(long)]
        save_dir: Option<PathBuf>,
        /// Open a window instead of fullscreen kiosk.
        #[arg(long)]
        windowed: bool,
    },
    /// Run the local Game Player host (library UI + game launch). Uses an installed Godot.
    Host {
        /// Giga Couch checkout or kit root containing sdk/ and tools/.
        #[arg(long)]
        root: PathBuf,
        /// Godot project path for the library UI. Defaults to <root>/sdk.
        #[arg(long)]
        sdk: Option<PathBuf>,
    },
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cli = Cli::parse();
    match run(&cli).await {
        Ok((value, human)) => {
            let (ok, error) = command_status(&cli.command, &value);
            if cli.json {
                let mut envelope = json!({ "ok": ok, "data": value });
                if let Some(error) = error {
                    envelope["error"] = error;
                }
                println!("{envelope}");
            } else {
                println!("{human}");
            }
            if !ok {
                std::process::exit(1);
            }
        }
        Err(error) => {
            let code = if let Some(e) = error.downcast_ref::<couch_manifests::Error>() {
                e.code()
            } else if let Some(e) = error.downcast_ref::<couch_local_library::Error>() {
                e.code()
            } else if let Some(e) = error.downcast_ref::<project::CliError>() {
                e.code()
            } else if let Some(e) = error.downcast_ref::<couch_web_host::Error>() {
                e.code()
            } else {
                "COMMAND_FAILED"
            };
            if cli.json {
                println!(
                    "{}",
                    json!({
                        "ok": false,
                        "error": { "code": code, "message": format!("{error:#}") }
                    })
                );
            } else {
                eprintln!("{code}: {error:#}");
            }
            std::process::exit(1);
        }
    }
}

async fn require_godot(cli: &Cli) -> Result<PathBuf> {
    let mut discovery = couch_runtime::Discovery::system(cli.godot.clone());
    discovery.probe_timeout = std::time::Duration::from_secs(cli.godot_timeout_secs);
    discovery.check().await.executable.ok_or_else(|| {
        project::CliError::new(
            "GODOT_NOT_READY",
            "pass --godot with a supported Godot executable; this command never downloads Godot or export templates",
        )
        .into()
    })
}

fn data_dir(cli: &Cli) -> Result<PathBuf> {
    if let Some(path) = &cli.data_dir {
        return Ok(path.clone());
    }
    directories::ProjectDirs::from("", "", "GigaCouch")
        .map(|dirs| dirs.data_local_dir().to_path_buf())
        .context("could not determine application directory; pass --data-dir")
}

fn command_status(command: &Command, value: &Value) -> (bool, Option<Value>) {
    if matches!(command, Command::Check { .. }) && value["supported"] != true {
        return (
            false,
            Some(json!({
                "code": "PROJECT_UNSUPPORTED",
                "message": "Project checks failed; see data.issues"
            })),
        );
    }
    let Command::Doctor { require_godot, .. } = command else {
        return (true, None);
    };
    if *require_godot && value["godot"]["supported"] != true {
        return (
            false,
            Some(json!({
                "code": "GODOT_NOT_READY",
                "message": "Install or select a supported Godot version; see data.godot.instructions"
            })),
        );
    }
    if let Some(project) = value.get("project") {
        let supported = project["supported"] == true;
        let only_nonfatal = project["issues"].as_array().is_none_or(|issues| {
            issues
                .iter()
                .all(|issue| issue["code"] == "COUCH_GAME_JSON_MISSING")
        });
        if !supported || !only_nonfatal {
            let error = project["issues"]
                .as_array()
                .and_then(|issues| {
                    issues
                        .iter()
                        .find(|issue| issue["code"] != "COUCH_GAME_JSON_MISSING")
                })
                .cloned()
                .unwrap_or_else(|| {
                    json!({
                        "code": "PROJECT_UNSUPPORTED",
                        "message": "This folder is not a supported Giga Couch project; see data.project.issues"
                    })
                });
            return (false, Some(error));
        }
    }
    (true, None)
}

async fn run(cli: &Cli) -> Result<(Value, String)> {
    match &cli.command {
        Command::Doctor { project, .. } => {
            let root = data_dir(cli)?;
            let mut discovery = couch_runtime::Discovery::system(cli.godot.clone());
            discovery.probe_timeout = std::time::Duration::from_secs(cli.godot_timeout_secs);
            let report = discovery.check().await;
            let mut godot_message = format!(
                "Godot: {} (required: {} standard stable)",
                report.status, report.policy.godot_version
            );
            if let Some(path) = &report.executable {
                godot_message.push_str(&format!("\nExecutable: {}", path.display()));
            }
            for attempt in &report.attempts {
                if !attempt.supported {
                    let found = attempt
                        .version
                        .as_ref()
                        .map(|version| format!(" (found {})", version.raw))
                        .unwrap_or_default();
                    godot_message.push_str(&format!(
                        "\n{}{found}: {}",
                        attempt.executable.display(),
                        attempt.reason
                    ));
                }
            }
            if !report.supported {
                godot_message.push_str("\nSetup instructions:\n");
                godot_message.push_str(&report.instructions.join("\n"));
            }
            let mut value = json!({
                "version": env!("CARGO_PKG_VERSION"),
                "data_dir": root,
                "host_target": Target::current(),
                "runtime_management": "not_implemented",
                "game_launch": "not_implemented",
                "godot_required_for_current_commands": false,
                "automatic_downloads": false,
                "godot": report
            });
            let mut human = format!(
                "Giga Couch {}\nData directory: {}\n{godot_message}\nPackage and library commands require no Godot.\n`couch web-serve` hosts a local web-1 package and does not download a browser.",
                env!("CARGO_PKG_VERSION"),
                root.display()
            );
            if let Some(project_path) = project {
                let project = project::inspect(project_path)?;
                value["project"] = project.to_value();
                human.push('\n');
                human.push_str(&project.human());
            }
            Ok((value, human))
        }
        Command::Init {
            title,
            parent,
            template,
        } => {
            let result = project::init(title, parent, template.as_deref(), &data_dir(cli)?)?;
            let path = result.path.to_string_lossy().into_owned();
            let value = json!({
                "path": path,
                "id": result.id,
                "title": result.title,
                "registered": true
            });
            Ok((
                value,
                format!("Created {path}.\nReopen Giga Couch to list it."),
            ))
        }
        Command::Check { project } => {
            let value = workflow::check(project)?;
            let supported = value["supported"] == true;
            Ok((
                value.clone(),
                if supported {
                    format!("Project checks passed: {}", value["path"])
                } else {
                    format!("Project checks failed: {}", value["path"])
                },
            ))
        }
        Command::Run { project } => {
            let godot = require_godot(cli).await?;
            let value = workflow::run_project(project, &godot, &data_dir(cli)?)?;
            Ok((
                value.clone(),
                format!("Godot exited {}", value["exit_code"]),
            ))
        }
        Command::Pack { project, target } => {
            let godot = require_godot(cli).await?;
            let selected = target
                .or_else(Target::current)
                .context("unsupported host target; pass --target")?;
            let value = workflow::pack(project, &godot, selected)?;
            Ok((
                value.clone(),
                format!("Packed {} ({} bytes).", value["pck"], value["bytes"]),
            ))
        }
        Command::Publish {
            project,
            visibility,
        } => {
            let value = workflow::publish_local(project, visibility, &data_dir(cli)?)?;
            Ok((
                value.clone(),
                format!("Private drop at {}.\nNo account upload.", value["path"]),
            ))
        }
        Command::Validate { manifest } => {
            let package = Manifest::read(manifest)?;
            package.verify_all(manifest.parent().unwrap_or_else(|| Path::new(".")))?;
            let value = json!({
                "game_id": package.game_id,
                "release_id": package.release_id,
                "artifacts_verified": package.artifacts.len(),
                "scope": "metadata_and_content_integrity",
                "playability_verified": false,
                "signature_verified": false
            });
            Ok((
                value,
                format!(
                    "Validated {} / {} ({} artifacts).\nMetadata and content integrity only; playability and signatures are not verified.",
                    package.game_id,
                    package.release_id,
                    package.artifacts.len()
                ),
            ))
        }
        Command::Install { manifest, target } => {
            let selected = target
                .or_else(Target::current)
                .context("unsupported host target; pass --target for a content-only import")?;
            // Fail invalid manifests before initializing an on-disk library.
            Manifest::read(manifest)?.artifact(selected)?;
            let library = Library::open(&data_dir(cli)?).await?;
            let outcome = library.install(manifest, selected).await;
            library.close().await;
            let result = outcome?;
            let human = format!(
                "{} {} / {} for {}.\nUnsigned local content only. No runtime was installed and no game was run.",
                if result.already_installed {
                    "Already installed"
                } else {
                    "Installed"
                },
                result.game_id,
                result.release_id,
                result.target
            );
            Ok((serde_json::to_value(result)?, human))
        }
        Command::WebHome {
            windowed,
            serve_only,
            platform,
        } => {
            web::home(
                if *serve_only {
                    web::HomeShell::ServeOnly
                } else {
                    web::HomeShell::Launch {
                        windowed: *windowed,
                    }
                },
                cli.json,
                &data_dir(cli)?,
                cli.godot.clone(),
                std::time::Duration::from_secs(cli.godot_timeout_secs),
                platform,
            )
            .await?;
            std::process::exit(0);
        }
        Command::Platform { bind } => {
            let data = data_dir(cli)?.join("platform");
            let packages = data.join("packages");
            let root = web::checkout_root()
                .context("could not find the game packages from this checkout")?;
            couch_platform::publish_blob_island(
                &packages,
                &root.join("runtimes/web/examples/blob-island"),
            )?;
            couch_platform::serve_until_stopped(&data, &packages, bind, None)?;
            std::process::exit(0);
        }
        Command::WebServe { package, save_dir } => {
            web::serve(package, save_dir.as_deref(), cli.json, &data_dir(cli)?)?;
            std::process::exit(0);
        }
        Command::WebRun {
            package,
            save_dir,
            windowed,
        } => {
            web::run(
                package,
                save_dir.as_deref(),
                *windowed,
                cli.json,
                &data_dir(cli)?,
            )?;
            std::process::exit(0);
        }
        Command::Host { root, sdk } => {
            let godot = if let Some(path) = &cli.godot {
                path.clone()
            } else {
                let mut discovery = couch_runtime::Discovery::system(None);
                discovery.probe_timeout = std::time::Duration::from_secs(cli.godot_timeout_secs);
                discovery
                    .check()
                    .await
                    .executable
                    .context("pass --godot with a supported Godot executable")?
            };
            let sdk = sdk.clone().unwrap_or_else(|| root.join("sdk"));
            let data = data_dir(cli)?;
            let startup = std::env::var_os("COUCH_PLAYER_STARTUP").map(PathBuf::from);
            let code = host::run(&godot, root, &sdk, &data, startup.as_deref()).await?;
            std::process::exit(code);
        }
        Command::Library => {
            let library = Library::open(&data_dir(cli)?).await?;
            let outcome = library.list().await;
            library.close().await;
            let releases = outcome?;
            let human = if releases.is_empty() {
                "No games installed.".into()
            } else {
                releases
                    .iter()
                    .map(|r| {
                        format!(
                            "{} {} / {} [{}] — {}",
                            if r.active { "*" } else { " " },
                            r.game_id,
                            r.release_id,
                            r.target,
                            r.title
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            Ok((json!({ "releases": releases }), human))
        }
    }
}
