use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use ytpm_core::{
    archive_project, create_project, expected_directories, list_projects, migrate_project,
    restore_project, validate_project, Asset, AssetCatalog, AssetKind, AssetState,
    CreateProjectRequest, Project, ProjectStatus, RecoveryReport, Task, TaskPatch, TaskPriority,
    TaskRequest, TaskStatus, YtpmError,
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
    Index {
        #[command(subcommand)]
        command: IndexCommand,
    },
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
    Asset {
        #[command(subcommand)]
        command: AssetCommand,
    },
    Document {
        #[command(subcommand)]
        command: DocumentCommand,
    },
    Journal {
        #[command(subcommand)]
        command: JournalCommand,
    },
    Timeline {
        #[command(subcommand)]
        command: TimelineCommand,
    },
    Media {
        #[command(subcommand)]
        command: MediaCommand,
    },
    Publish {
        #[command(subcommand)]
        command: PublishCommand,
    },
}

#[derive(Debug, Subcommand)]
enum IndexCommand {
    Rebuild {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Search {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        query: Option<String>,
        #[arg(long, value_parser = parse_project_status)]
        status: Option<ProjectStatus>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum TaskCommand {
    List {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Create {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        title: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long, value_parser = parse_task_status, default_value = "todo")]
        status: TaskStatus,
        #[arg(long, value_parser = parse_task_priority, default_value = "normal")]
        priority: TaskPriority,
        #[arg(long, default_value_t = 0.0)]
        order_key: f64,
        #[arg(long)]
        due_at: Option<String>,
        #[arg(long, value_delimiter = ',')]
        related_asset_ids: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        acceptance_criteria: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    Update {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        clear_description: bool,
        #[arg(long, value_parser = parse_task_priority)]
        priority: Option<TaskPriority>,
        #[arg(long)]
        due_at: Option<String>,
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        acceptance_criteria: Option<Vec<String>>,
        #[arg(long)]
        json: bool,
    },
    Move {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        task_id: String,
        #[arg(long, value_parser = parse_task_status)]
        status: TaskStatus,
        #[arg(long)]
        order_key: f64,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum AssetCommand {
    Scan {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    List {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum DocumentCommand {
    Read {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        relative_path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Write {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        relative_path: PathBuf,
        #[arg(long)]
        content: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum JournalCommand {
    Recover {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum TimelineCommand {
    Load {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Validate {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum MediaCommand {
    Probe {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        relative_path: String,
        #[arg(long)]
        json: bool,
    },
    Export {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        output: String,
        #[arg(long)]
        confirm: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum PublishCommand {
    Config {
        #[arg(long)]
        json: bool,
    },
    DryRun {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Upload {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        confirm: bool,
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
            print_projects(&projects, json)?;
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
        Command::Index { command } => match command {
            IndexCommand::Rebuild { root, json } => {
                let report = ytpm_core::rebuild_index(&root)
                    .with_context(|| format!("無法重建 Library index：{}", root.display()))?;
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "db_path": report.db_path,
                            "scanned": report.scanned,
                            "indexed": report.indexed,
                            "invalid": report.invalid,
                            "rebuilt_at": null
                        })
                    );
                } else {
                    println!("索引已重建：{}", report.db_path.display());
                    println!(
                        "掃描：{} · 索引：{} · invalid：{}",
                        report.scanned, report.indexed, report.invalid
                    );
                }
            }
            IndexCommand::Search {
                root,
                query,
                status,
                json,
            } => {
                ytpm_core::recover_operation_journal(&root)?;
                let projects = ytpm_core::search_index(&root, query.as_deref(), status)?;
                print_projects(&projects, json)?;
            }
        },
        Command::Task { command } => match command {
            TaskCommand::List { path, json } => {
                let tasks = ytpm_core::list_tasks(&path)?;
                print_tasks(&tasks, json)?;
            }
            TaskCommand::Create {
                path,
                title,
                description,
                status,
                priority,
                order_key,
                due_at,
                related_asset_ids,
                acceptance_criteria,
                json,
            } => {
                let request = task_request(
                    title,
                    description,
                    status,
                    priority,
                    order_key,
                    due_at,
                    related_asset_ids,
                    acceptance_criteria,
                )?;
                let task = ytpm_core::create_task(&path, request)
                    .with_context(|| format!("無法在 {} 建立任務", path.display()))?;
                print_task(&task, "已建立任務", json)?;
            }
            TaskCommand::Update {
                path,
                task_id,
                title,
                description,
                clear_description,
                priority,
                due_at,
                acceptance_criteria,
                json,
            } => {
                let patch = task_patch(
                    title,
                    description,
                    clear_description,
                    priority,
                    due_at,
                    acceptance_criteria,
                )?;
                let task = ytpm_core::update_task(&path, &task_id, patch)
                    .with_context(|| format!("無法更新任務 {task_id}"))?;
                print_task(&task, "已更新任務", json)?;
            }
            TaskCommand::Move {
                path,
                task_id,
                status,
                order_key,
                json,
            } => {
                let task = ytpm_core::move_task(&path, &task_id, status, order_key)
                    .with_context(|| format!("無法移動任務 {task_id}"))?;
                print_task(&task, "已移動任務", json)?;
            }
        },
        Command::Asset { command } => match command {
            AssetCommand::Scan { path, json } => {
                let catalog = ytpm_core::scan_assets(&path)
                    .with_context(|| format!("無法掃描素材：{}", path.display()))?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&catalog)?);
                } else {
                    print_asset_summary(&catalog, &path);
                }
            }
            AssetCommand::List { path, json } => {
                let assets = ytpm_core::list_assets(&path)?;
                print_assets(&assets, json)?;
            }
        },
        Command::Document { command } => match command {
            DocumentCommand::Read {
                path,
                relative_path,
                json,
            } => {
                let content = ytpm_core::read_document(&path, &relative_path)?;
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "relative_path": relative_path,
                            "content": content,
                            "saved_at": null
                        })
                    );
                } else {
                    print!("{content}");
                }
            }
            DocumentCommand::Write {
                path,
                relative_path,
                content,
                json,
            } => {
                ytpm_core::write_document(&path, &relative_path, &content)?;
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "relative_path": relative_path,
                            "saved": true
                        })
                    );
                } else {
                    println!("已儲存文件：{}", path.join(relative_path).display());
                }
            }
        },
        Command::Journal { command } => match command {
            JournalCommand::Recover { root, json } => {
                let report = ytpm_core::recover_operation_journal(&root)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    print_recovery_report(&report, &root);
                }
            }
        },
        Command::Timeline { command } => match command {
            TimelineCommand::Load { path, json } => {
                let timeline = ytpm_core::read_timeline(&path)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&timeline)?);
                } else {
                    println!(
                        "timeline：{} ms · tracks：{} · updated：{}",
                        timeline.duration_ms,
                        timeline.tracks.len(),
                        timeline.updated_at
                    );
                }
            }
            TimelineCommand::Validate { path, json } => {
                let timeline = ytpm_core::read_timeline(&path)?;
                let report = ytpm_core::validate_timeline(&timeline);
                if json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    println!(
                        "timeline valid：{} · issues：{}",
                        report.valid,
                        report.issues.len()
                    );
                }
                if !report.valid {
                    std::process::exit(12);
                }
            }
        },
        Command::Media { command } => match command {
            MediaCommand::Probe {
                path,
                relative_path,
                json,
            } => {
                let probe = ytpm_core::probe_media(&path, &relative_path)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&probe)?);
                } else {
                    println!(
                        "{} · {:?} · {:?}s",
                        probe.relative_path, probe.format_name, probe.duration_seconds
                    );
                }
            }
            MediaCommand::Export {
                path,
                output,
                confirm,
                json,
            } => {
                if !confirm {
                    anyhow::bail!("media export 需要 --confirm；先確認輸出路徑與 timeline");
                }
                let timeline = ytpm_core::read_timeline(&path)?;
                let result = ytpm_core::export_timeline(&path, &timeline, &output, None)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!(
                        "FFmpeg export：{:?} · {}",
                        result.status,
                        result.message.unwrap_or_default()
                    );
                }
            }
        },
        Command::Publish { command } => match command {
            PublishCommand::Config { json } => {
                let config = ytpm_core::config_reference();
                if json {
                    println!("{}", serde_json::to_string_pretty(&config)?);
                } else {
                    println!(
                        "provider：{} · oauth_ready：{} · config：{}",
                        config.provider, config.oauth_ready, config.config_path
                    );
                }
            }
            PublishCommand::DryRun { path, json } => {
                let metadata = ytpm_core::load_publish_metadata(&path)?;
                let result = ytpm_core::publish_dry_run(&path, &metadata)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("publish dry-run：{:?} · {}", result.status, result.message);
                }
            }
            PublishCommand::Upload {
                path,
                confirm,
                json,
            } => {
                if !confirm {
                    anyhow::bail!("publish upload 需要 --confirm；先執行 publish dry-run");
                }
                let metadata = ytpm_core::load_publish_metadata(&path)?;
                let result = ytpm_core::upload_video(&path, &metadata, None)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("publish upload：{:?} · {}", result.status, result.message);
                }
            }
        },
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn task_request(
    title: String,
    description: Option<String>,
    status: TaskStatus,
    priority: TaskPriority,
    order_key: f64,
    due_at: Option<String>,
    related_asset_ids: Vec<String>,
    acceptance_criteria: Vec<String>,
) -> Result<TaskRequest> {
    let mut value = serde_json::json!({
        "title": title,
        "description": description,
        "status": status,
        "priority": priority,
        "order_key": order_key,
        "related_asset_ids": related_asset_ids,
        "acceptance_criteria": acceptance_criteria
    });
    if let Some(due_at) = due_at {
        value["due_at"] = serde_json::Value::String(due_at);
    }
    serde_json::from_value(value).context("任務輸入格式無效")
}

fn task_patch(
    title: Option<String>,
    description: Option<String>,
    clear_description: bool,
    priority: Option<TaskPriority>,
    due_at: Option<String>,
    acceptance_criteria: Option<Vec<String>>,
) -> Result<TaskPatch> {
    let mut value = serde_json::Map::new();
    if let Some(title) = title {
        value.insert("title".into(), serde_json::Value::String(title));
    }
    if clear_description {
        value.insert("description".into(), serde_json::Value::Null);
    } else if let Some(description) = description {
        value.insert("description".into(), serde_json::Value::String(description));
    }
    if let Some(priority) = priority {
        value.insert("priority".into(), serde_json::to_value(priority)?);
    }
    if let Some(due_at) = due_at {
        value.insert("due_at".into(), serde_json::Value::String(due_at));
    }
    if let Some(acceptance_criteria) = acceptance_criteria {
        value.insert(
            "acceptance_criteria".into(),
            serde_json::to_value(acceptance_criteria)?,
        );
    }
    serde_json::from_value(serde_json::Value::Object(value)).context("任務 patch 格式無效")
}

fn print_projects(projects: &[Project], json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(projects)?);
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
    Ok(())
}

fn print_tasks(tasks: &[Task], json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(tasks)?);
    } else if tasks.is_empty() {
        println!("沒有找到任務。");
    } else {
        for task in tasks {
            println!(
                "{}\t{}\t{}\t{}",
                task.id,
                task_status_label(task.status),
                task_priority_label(task.priority),
                task.title
            );
        }
    }
    Ok(())
}

fn print_task(task: &Task, action: &str, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(task)?);
    } else {
        println!("{}：{}", action, task.title);
        println!("id：{}", task.id);
        println!("狀態：{}", task_status_label(task.status));
    }
    Ok(())
}

fn print_assets(assets: &[Asset], json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(assets)?);
    } else if assets.is_empty() {
        println!("沒有找到素材。");
    } else {
        for asset in assets {
            println!(
                "{}\t{}\t{}",
                asset_state_label(&asset.state),
                asset_kind_label(&asset.kind),
                asset.relative_path
            );
        }
    }
    Ok(())
}

fn print_asset_summary(catalog: &AssetCatalog, path: &Path) {
    let available = catalog
        .assets
        .iter()
        .filter(|asset| matches!(asset.state, AssetState::Available))
        .count();
    let missing = catalog
        .assets
        .iter()
        .filter(|asset| matches!(asset.state, AssetState::Missing))
        .count();
    let invalid = catalog
        .assets
        .iter()
        .filter(|asset| matches!(asset.state, AssetState::Error))
        .count();
    println!("素材掃描完成：{}", path.display());
    println!(
        "總數：{} · available：{} · missing：{} · invalid：{}",
        catalog.assets.len(),
        available,
        missing,
        invalid
    );
}

fn print_recovery_report(report: &RecoveryReport, root: &Path) {
    if report.journal_found && report.journal_cleared {
        println!(
            "已完成 journal recovery：{}",
            root.join(".ytpm-operation.json").display()
        );
        println!(
            "operation：{}",
            report.operation.as_deref().unwrap_or("unknown")
        );
        println!("phase：{}", report.phase.as_deref().unwrap_or("unknown"));
    } else if report.journal_found {
        println!("找到 journal，但尚未清除，請人工檢查。");
    } else {
        println!("沒有待恢復的操作 journal。");
    }
}

fn parse_project_status(value: &str) -> std::result::Result<ProjectStatus, String> {
    match value {
        "idea" => Ok(ProjectStatus::Idea),
        "research" => Ok(ProjectStatus::Research),
        "script" => Ok(ProjectStatus::Script),
        "voice" => Ok(ProjectStatus::Voice),
        "visuals" => Ok(ProjectStatus::Visuals),
        "editing" => Ok(ProjectStatus::Editing),
        "subtitles" => Ok(ProjectStatus::Subtitles),
        "thumbnail" => Ok(ProjectStatus::Thumbnail),
        "review" => Ok(ProjectStatus::Review),
        "scheduled" => Ok(ProjectStatus::Scheduled),
        "published" => Ok(ProjectStatus::Published),
        "archived" => Ok(ProjectStatus::Archived),
        _ => Err(format!("不支援的 project status：{value}")),
    }
}

fn parse_task_status(value: &str) -> std::result::Result<TaskStatus, String> {
    match value {
        "todo" => Ok(TaskStatus::Todo),
        "doing" => Ok(TaskStatus::Doing),
        "review" => Ok(TaskStatus::Review),
        "blocked" => Ok(TaskStatus::Blocked),
        "done" => Ok(TaskStatus::Done),
        _ => Err(format!("不支援的 task status：{value}")),
    }
}

fn parse_task_priority(value: &str) -> std::result::Result<TaskPriority, String> {
    match value {
        "low" => Ok(TaskPriority::Low),
        "normal" => Ok(TaskPriority::Normal),
        "high" => Ok(TaskPriority::High),
        "urgent" => Ok(TaskPriority::Urgent),
        _ => Err(format!("不支援的 task priority：{value}")),
    }
}

fn task_status_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "todo",
        TaskStatus::Doing => "doing",
        TaskStatus::Review => "review",
        TaskStatus::Blocked => "blocked",
        TaskStatus::Done => "done",
    }
}

fn task_priority_label(priority: TaskPriority) -> &'static str {
    match priority {
        TaskPriority::Low => "low",
        TaskPriority::Normal => "normal",
        TaskPriority::High => "high",
        TaskPriority::Urgent => "urgent",
    }
}

fn asset_kind_label(kind: &AssetKind) -> &'static str {
    match kind {
        AssetKind::Research => "research",
        AssetKind::Script => "script",
        AssetKind::Voice => "voice",
        AssetKind::Music => "music",
        AssetKind::SoundEffect => "sound_effect",
        AssetKind::Image => "image",
        AssetKind::Video => "video",
        AssetKind::Subtitle => "subtitle",
        AssetKind::Thumbnail => "thumbnail",
        AssetKind::Metadata => "metadata",
        AssetKind::Export => "export",
        AssetKind::Other => "other",
    }
}

fn asset_state_label(state: &AssetState) -> &'static str {
    match state {
        AssetState::Available => "available",
        AssetState::Missing => "missing",
        AssetState::Archived => "archived",
        AssetState::Processing => "processing",
        AssetState::Error => "error",
    }
}

fn exit_code(error: &anyhow::Error) -> i32 {
    match error.downcast_ref::<YtpmError>() {
        Some(YtpmError::InvalidInput(_)) => 2,
        Some(YtpmError::Io { .. }) => 10,
        Some(YtpmError::InvalidProject(_)) | Some(YtpmError::Json(_)) => 11,
        None => 20,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_nested_index_search_command() {
        let cli = Cli::try_parse_from([
            "ytpm",
            "index",
            "search",
            "--root",
            "D:\\library",
            "--query",
            "AI",
            "--status",
            "review",
            "--json",
        ])
        .expect("index search should parse");
        assert!(matches!(
            cli.command,
            Command::Index {
                command: IndexCommand::Search {
                    status: Some(ProjectStatus::Review),
                    json: true,
                    ..
                }
            }
        ));
    }

    #[test]
    fn task_patch_supports_clearing_description() {
        let patch = task_patch(None, None, true, None, None, None).expect("patch should parse");
        assert_eq!(patch.description, Some(None));
    }
}
