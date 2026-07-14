use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use ytpm_core::{
    archive_project, create_project, expected_directories, list_projects, migrate_project,
    restore_project, validate_project, CreateProjectRequest, YtpmError,
};

#[derive(Debug, Parser)]
#[command(name = "ytpm", version, about = "YouTube Project Manager CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Create {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        title: String,
        #[arg(long)]
        channel: Option<String>,
        #[arg(long, default_value = "zh-TW")]
        language: String,
        #[arg(long, default_value = "16:9")]
        aspect_ratio: String,
        #[arg(long)]
        json: bool,
    },
    List {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Validate {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Structure {
        #[arg(long)]
        json: bool,
    },
    Archive {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Restore {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Migrate {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(exit_code(&error));
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Create {
            root,
            title,
            channel,
            language,
            aspect_ratio,
            json,
        } => {
            let project = create_project(
                &root,
                CreateProjectRequest {
                    title,
                    channel,
                    series: None,
                    aspect_ratio,
                    language,
                    target_duration_seconds: None,
                    planned_publish_at: None,
                    tags: Vec::new(),
                },
            )
            .with_context(|| format!("無法在 {} 建立專案", root.display()))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&project)?);
            } else {
                println!("已建立：{}", project.title);
                println!("資料夾：{}", root.join(project.folder_name).display());
            }
        }
        Command::List { root, json } => {
            let projects = list_projects(&root)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&projects)?);
            } else if projects.is_empty() {
                println!("沒有找到影片專案。");
            } else {
                for project in projects {
                    println!(
                        "{}\t{:?}\t{}%",
                        project.title, project.status, project.progress
                    );
                }
            }
        }
        Command::Validate { path, json } => {
            let report = validate_project(&path)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("有效：{}", report.valid);
                for issue in report.issues {
                    println!("[{:?}] {} — {}", issue.severity, issue.code, issue.message);
                }
            }
            if !report.valid {
                std::process::exit(12);
            }
        }
        Command::Structure { json } => {
            if json {
                println!("{}", serde_json::to_string_pretty(expected_directories())?);
            } else {
                for directory in expected_directories() {
                    println!("{directory}");
                }
            }
        }
        Command::Archive { path, json } => {
            let project = archive_project(&path)
                .with_context(|| format!("無法封存專案 {}", path.display()))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&project)?);
            } else {
                println!("已封存：{}", project.title);
                println!("狀態：{:?}", project.status);
            }
        }
        Command::Restore { path, json } => {
            let project = restore_project(&path)
                .with_context(|| format!("無法還原專案 {}", path.display()))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&project)?);
            } else {
                println!("已還原：{}", project.title);
                println!("狀態：{:?}", project.status);
            }
        }
        Command::Migrate { path, json } => {
            let project = migrate_project(&path)
                .with_context(|| format!("無法遷移專案 {}", path.display()))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&project)?);
            } else {
                println!("已遷移：{}", project.title);
                println!("schema_version：{}", project.schema_version);
            }
        }
    }
    Ok(())
}

fn exit_code(error: &anyhow::Error) -> i32 {
    match error.downcast_ref::<YtpmError>() {
        Some(YtpmError::InvalidInput(_)) => 2,
        Some(YtpmError::Io { .. }) => 10,
        Some(YtpmError::InvalidProject(_)) | Some(YtpmError::Json(_)) => 11,
        None => 20,
    }
}
