//! CALIBER TUI entry point.

use caliber_api::types::{
    CompileDslRequest, DeployDslRequest, ListAgentsRequest, ListArtifactsRequest,
    ListMessagesRequest, ListNotesRequest, ListTrajectoriesRequest, PackSource, PackSourceFile,
    ValidateDslRequest,
};
use caliber_core::{EntityIdType, ScopeId, TenantId, TrajectoryId};
use caliber_dsl::parser::CaliberAst;
use caliber_dsl::pretty_printer::pretty_print;
use caliber_tui::api_client::ApiClient;
use caliber_tui::config::TuiConfig;
use caliber_tui::error::TuiError;
use caliber_tui::events::TuiEvent;
use caliber_tui::keys::{map_key, KeyAction};
use caliber_tui::persistence::{self, PersistedState};
use caliber_tui::state::App;
use caliber_tui::views::render_view;
use crossterm::{
    event::{self, Event as CrosstermEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::fs;
use std::io::{self, Stdout};
use std::path::Path;
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), TuiError> {
    let config = TuiConfig::load()?;
    let api = ApiClient::new(&config)?;
    let mut app = App::new(config, api);
    app.config_view.content = format!("{:#?}", app.config);
    if let Ok(Some(state)) = persistence::load(&app.config.persistence_path) {
        app.active_view = state.active_view;
        if state.selected_tenant_id == Some(app.tenant.tenant_id) {
            app.tenant_view.selected = state.selected_tenant_id.map(|id| id.as_uuid());
        }
    }

    let mut terminal = setup_terminal()?;
    let _guard = TerminalGuard {};

    let (event_tx, mut event_rx) = mpsc::channel::<TuiEvent>(256);

    spawn_input_reader(event_tx.clone());
    initialize_app(&mut app, event_tx.clone()).await;
    if let Err(err) = refresh_view(&mut app).await {
        app.notify(
            caliber_tui::notifications::NotificationLevel::Error,
            format!("Initial refresh failed: {}", err),
        );
    }
    caliber_tui::realtime::spawn_ws_manager(
        app.api.ws().clone(),
        app.tenant.tenant_id,
        event_tx.clone(),
    );

    let tick_rate = Duration::from_millis(app.config.refresh_interval_ms);
    let mut ticker = tokio::time::interval(tick_rate);

    loop {
        terminal.draw(|f| render_view(f, &app))?;

        tokio::select! {
            _ = ticker.tick() => {
                if !app.updates_paused {
                    app.flush_queued_events();
                }
                let _ = event_tx.send(TuiEvent::Tick).await;
            }
            Some(event) = event_rx.recv() => {
                if handle_event(&mut app, event).await? {
                    break;
                }
            }
        }
    }

    let persisted = PersistedState {
        active_view: app.active_view,
        selected_tenant_id: app.tenant_view.selected.map(TenantId::new),
    };
    let _ = persistence::save(&app.config.persistence_path, &persisted);

    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, TuiError> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen);
    }
}

fn spawn_input_reader(sender: mpsc::Sender<TuiEvent>) {
    std::thread::spawn(move || loop {
        if let Ok(true) = event::poll(Duration::from_millis(200)) {
            if let Ok(evt) = event::read() {
                match evt {
                    CrosstermEvent::Key(key) => {
                        let _ = sender.blocking_send(TuiEvent::Input(key));
                    }
                    CrosstermEvent::Resize(width, height) => {
                        let _ = sender.blocking_send(TuiEvent::Resize { width, height });
                    }
                    _ => {}
                }
            }
        }
    });
}

async fn initialize_app(app: &mut App, sender: mpsc::Sender<TuiEvent>) {
    match app.api.rest().list_tenants().await {
        Ok(response) => {
            app.tenant.available_tenants = response.tenants.clone();
            app.tenant_view.tenants = response.tenants;
            if let Some(tenant) = app
                .tenant_view
                .tenants
                .iter()
                .find(|t| t.tenant_id == app.tenant.tenant_id)
            {
                app.tenant_view.selected = Some(tenant.tenant_id.as_uuid());
                app.tenant.tenant_name = tenant.name.clone();
            }
        }
        Err(err) => {
            let _ = sender
                .send(TuiEvent::ApiError(format!("Tenant load failed: {}", err)))
                .await;
        }
    }
}

async fn handle_event(app: &mut App, event: TuiEvent) -> Result<bool, TuiError> {
    match event {
        TuiEvent::Input(key) => {
            if let Some(action) = map_key(key) {
                return handle_action(app, action).await;
            }
        }
        TuiEvent::Ws(ws_event) => {
            app.enqueue_event(*ws_event);
        }
        TuiEvent::ApiError(message) => {
            app.notify(
                caliber_tui::notifications::NotificationLevel::Error,
                message,
            );
        }
        TuiEvent::Resize { .. } | TuiEvent::Tick => {}
    }
    Ok(false)
}

async fn handle_action(app: &mut App, action: KeyAction) -> Result<bool, TuiError> {
    match action {
        KeyAction::Quit => return Ok(true),
        KeyAction::NextView => app.active_view = app.active_view.next(),
        KeyAction::PrevView => app.active_view = app.active_view.previous(),
        KeyAction::SwitchView(index) => {
            if let Some(view) = caliber_tui::nav::View::from_index(index) {
                app.active_view = view;
            }
        }
        KeyAction::MoveDown => app.select_next(),
        KeyAction::MoveUp => app.select_previous(),
        KeyAction::ToggleExpand => app.toggle_expand(),
        KeyAction::PauseUpdates => {
            app.updates_paused = !app.updates_paused;
            if !app.updates_paused {
                app.flush_queued_events();
            }
        }
        KeyAction::Refresh => {
            if app.active_view == caliber_tui::nav::View::DslEditor {
                refresh_dsl_view(app).await?;
            } else {
                refresh_view(app).await?;
            }
        }
        KeyAction::OpenHelp => {
            app.modal = Some(caliber_tui::state::Modal {
                title: "Keybindings".to_string(),
                message: "Use h/j/k/l or arrows to move, Tab to switch views, q to quit."
                    .to_string(),
            })
        }
        KeyAction::OpenSearch => {
            app.search = Some(caliber_tui::state::GlobalSearch {
                query: String::new(),
            })
        }
        KeyAction::OpenCommand => {
            app.command_palette = Some(caliber_tui::state::CommandPalette {
                input: String::new(),
                suggestions: Vec::new(),
            })
        }
        KeyAction::NewItem => {
            if app.active_view == caliber_tui::nav::View::DslEditor {
                compose_pack_action(app).await?;
            } else {
                app.notify(
                    caliber_tui::notifications::NotificationLevel::Info,
                    "KeyAction queued.",
                );
            }
        }
        KeyAction::EditItem => {
            if app.active_view == caliber_tui::nav::View::DslEditor {
                compile_dsl_action(app).await?;
            } else {
                app.notify(
                    caliber_tui::notifications::NotificationLevel::Info,
                    "KeyAction queued.",
                );
            }
        }
        KeyAction::DeleteItem => {
            if app.active_view == caliber_tui::nav::View::DslEditor {
                deploy_dsl_action(app).await?;
            } else {
                app.notify(
                    caliber_tui::notifications::NotificationLevel::Info,
                    "KeyAction queued.",
                );
            }
        }
        KeyAction::Confirm => {
            if app.active_view == caliber_tui::nav::View::DslEditor {
                validate_dsl_action(app).await?;
            }
        }
        KeyAction::ToggleLinks => app.toggle_links_panel(),
        KeyAction::NextLink => app.next_link(),
        KeyAction::PrevLink => app.prev_link(),
        KeyAction::ExecuteLink => {
            if let Some(action) = app.selected_link_action().cloned() {
                let tenant_id = app.tenant.tenant_id;
                let is_destructive = action.is_destructive();

                // Confirm destructive actions
                if is_destructive {
                    app.modal = Some(caliber_tui::state::Modal {
                        title: "Confirm Delete".to_string(),
                        message: format!("Execute {} {}?", action.method(), action.link.href),
                    });
                    // TODO: Add confirmation flow - for now, skip destructive actions
                    return Ok(false);
                }

                app.notify(
                    caliber_tui::notifications::NotificationLevel::Info,
                    format!("Executing {} {}...", action.method(), action.link.href),
                );

                match app.api.rest().follow_link(tenant_id, &action.link).await {
                    Ok(_response) => {
                        app.notify(
                            caliber_tui::notifications::NotificationLevel::Info,
                            format!("{} completed", action.title()),
                        );
                        // Refresh current view to show updated data
                        refresh_view(app).await?;
                    }
                    Err(e) => {
                        app.notify(
                            caliber_tui::notifications::NotificationLevel::Error,
                            format!("Failed: {}", e),
                        );
                    }
                }
            }
        }
        KeyAction::Cancel | KeyAction::Select | KeyAction::MoveLeft | KeyAction::MoveRight => {}
    }
    Ok(false)
}

async fn refresh_view(app: &mut App) -> Result<(), TuiError> {
    let tenant_id = app.tenant.tenant_id;
    match app.active_view {
        caliber_tui::nav::View::TrajectoryTree => {
            if app.trajectory_view.filter.status.is_none() {
                app.trajectory_view.filter.status = Some(caliber_core::TrajectoryStatus::Active);
            }
            let params = ListTrajectoriesRequest {
                status: app.trajectory_view.filter.status,
                agent_id: app.trajectory_view.filter.agent_id,
                parent_id: None,
                limit: None,
                offset: None,
            };
            let response = app.api.rest().list_trajectories(tenant_id, &params).await?;
            app.trajectory_view.trajectories = response.trajectories;
        }
        caliber_tui::nav::View::ScopeExplorer => {
            if let Some(trajectory_id) = app.trajectory_view.selected {
                let scopes = app
                    .api
                    .rest()
                    .list_scopes(tenant_id, TrajectoryId::new(trajectory_id))
                    .await?;
                app.scope_view.scopes = scopes;
            }
        }
        caliber_tui::nav::View::ArtifactBrowser => {
            let response = app
                .api
                .rest()
                .list_artifacts(
                    tenant_id,
                    &ListArtifactsRequest {
                        artifact_type: app.artifact_view.filter.artifact_type,
                        trajectory_id: app.artifact_view.filter.trajectory_id,
                        scope_id: app.artifact_view.filter.scope_id,
                        created_after: app.artifact_view.filter.date_from,
                        created_before: app.artifact_view.filter.date_to,
                        limit: None,
                        offset: None,
                    },
                )
                .await?;
            app.artifact_view.artifacts = response.artifacts;
        }
        caliber_tui::nav::View::NoteLibrary => {
            let response = app
                .api
                .rest()
                .list_notes(
                    tenant_id,
                    &ListNotesRequest {
                        note_type: app.note_view.filter.note_type,
                        source_trajectory_id: app.note_view.filter.source_trajectory_id,
                        created_after: app.note_view.filter.date_from,
                        created_before: app.note_view.filter.date_to,
                        limit: None,
                        offset: None,
                    },
                )
                .await?;
            app.note_view.notes = response.notes;
        }
        caliber_tui::nav::View::TurnHistory => {
            if let Some(scope_id) = app.scope_view.selected {
                let turns = app
                    .api
                    .rest()
                    .list_turns(tenant_id, ScopeId::new(scope_id))
                    .await?;
                app.turn_view.turns = turns;
            }
        }
        caliber_tui::nav::View::AgentDashboard => {
            let response = app
                .api
                .rest()
                .list_agents(
                    tenant_id,
                    &ListAgentsRequest {
                        agent_type: app.agent_view.filter.agent_type.clone(),
                        status: app.agent_view.filter.status.clone(),
                        trajectory_id: None,
                        active_only: None,
                    },
                )
                .await?;
            app.agent_view.agents = response.agents;
        }
        caliber_tui::nav::View::LockMonitor => {
            let response = app.api.rest().list_locks(tenant_id).await?;
            app.lock_view.locks = response.locks;
        }
        caliber_tui::nav::View::MessageQueue => {
            let response = app
                .api
                .rest()
                .list_messages(
                    tenant_id,
                    &ListMessagesRequest {
                        message_type: app.message_view.filter.message_type,
                        from_agent_id: app.message_view.filter.from_agent_id,
                        to_agent_id: app.message_view.filter.to_agent_id,
                        to_agent_type: None,
                        trajectory_id: None,
                        priority: app.message_view.filter.priority,
                        undelivered_only: None,
                        unacknowledged_only: None,
                        limit: None,
                        offset: None,
                    },
                )
                .await?;
            app.message_view.messages = response.messages;
        }
        caliber_tui::nav::View::TenantManagement => {
            let response = app.api.rest().list_tenants().await?;
            app.tenant_view.tenants = response.tenants;
        }
        caliber_tui::nav::View::DslEditor | caliber_tui::nav::View::ConfigViewer => {}
    }
    Ok(())
}

async fn refresh_dsl_view(app: &mut App) -> Result<(), TuiError> {
    compose_pack_action(app).await
}

async fn validate_dsl_action(app: &mut App) -> Result<(), TuiError> {
    let tenant_id = app.tenant.tenant_id;
    let source = app.dsl_view.content.clone();
    if source.trim().is_empty() {
        app.notify(
            caliber_tui::notifications::NotificationLevel::Warning,
            "DSL content is empty; nothing to validate.",
        );
        return Ok(());
    }

    match app
        .api
        .rest()
        .validate_dsl(tenant_id, &ValidateDslRequest { source })
        .await
    {
        Ok(response) => {
            app.dsl_view.parse_errors = response.errors.clone();
            app.dsl_view.ast_preview = response.ast.clone();
            let level = if response.valid {
                caliber_tui::notifications::NotificationLevel::Success
            } else {
                caliber_tui::notifications::NotificationLevel::Error
            };
            let message = if response.valid {
                "DSL is valid."
            } else {
                "DSL validation failed."
            };
            app.notify(level, message);
        }
        Err(err) => {
            app.notify(
                caliber_tui::notifications::NotificationLevel::Error,
                format!("DSL validation error: {}", err),
            );
        }
    }

    Ok(())
}

async fn compile_dsl_action(app: &mut App) -> Result<(), TuiError> {
    let tenant_id = app.tenant.tenant_id;
    let source = app.dsl_view.content.clone();
    if source.trim().is_empty() {
        app.notify(
            caliber_tui::notifications::NotificationLevel::Warning,
            "DSL content is empty; nothing to compile.",
        );
        return Ok(());
    }

    match app
        .api
        .rest()
        .compile_dsl(tenant_id, &CompileDslRequest { source })
        .await
    {
        Ok(response) => {
            if response.success {
                app.notify(
                    caliber_tui::notifications::NotificationLevel::Success,
                    "DSL compiled successfully.",
                );
            } else {
                app.dsl_view.parse_errors = response
                    .errors
                    .iter()
                    .map(|e| caliber_api::types::ParseErrorResponse {
                        line: 0,
                        column: 0,
                        message: format!("{}: {}", e.error_type, e.message),
                    })
                    .collect();
                app.notify(
                    caliber_tui::notifications::NotificationLevel::Error,
                    "DSL compilation failed.",
                );
            }
        }
        Err(err) => {
            app.notify(
                caliber_tui::notifications::NotificationLevel::Error,
                format!("DSL compilation error: {}", err),
            );
        }
    }

    Ok(())
}

async fn compose_pack_action(app: &mut App) -> Result<(), TuiError> {
    if let Err(err) = compose_pack_inner(app).await {
        app.notify(
            caliber_tui::notifications::NotificationLevel::Error,
            format!("Pack compose failed: {}", err),
        );
    }
    Ok(())
}

async fn compose_pack_inner(app: &mut App) -> Result<(), TuiError> {
    let tenant_id = app.tenant.tenant_id;
    let Some(pack_root) = app.dsl_view.pack_root.clone() else {
        app.notify(
            caliber_tui::notifications::NotificationLevel::Warning,
            "No pack root configured. Expected ./agents-pack",
        );
        return Ok(());
    };

    let pack_source = pack_source_from_root(&pack_root)?;
    let markdowns: Vec<(String, String)> = pack_source
        .markdowns
        .iter()
        .map(|m| (m.path.clone(), m.content.clone()))
        .collect();

    let response = app
        .api
        .rest()
        .compose_pack(tenant_id, &pack_source.manifest, &markdowns)
        .await?;

    app.dsl_view.parse_errors = response
        .errors
        .iter()
        .map(|e| caliber_api::types::ParseErrorResponse {
            line: e.line,
            column: e.column,
            message: format!("{}: {}", e.file, e.message),
        })
        .collect();
    app.dsl_view.ast_preview = response.ast.clone();

    if response.success {
        if let Some(ast_value) = &response.ast {
            if let Some(dsl) = dsl_from_ast_value(ast_value) {
                app.dsl_view.content = dsl;
            }
        }
        app.notify(
            caliber_tui::notifications::NotificationLevel::Success,
            "Pack composed successfully.",
        );
    } else {
        app.notify(
            caliber_tui::notifications::NotificationLevel::Error,
            "Pack composition reported errors.",
        );
    }

    Ok(())
}

async fn deploy_dsl_action(app: &mut App) -> Result<(), TuiError> {
    if let Err(err) = deploy_dsl_inner(app).await {
        app.notify(
            caliber_tui::notifications::NotificationLevel::Error,
            format!("DSL deploy failed: {}", err),
        );
    }
    Ok(())
}

async fn deploy_dsl_inner(app: &mut App) -> Result<(), TuiError> {
    let tenant_id = app.tenant.tenant_id;
    let config_name = app.dsl_view.config_name.clone();

    let request = if let Some(pack_root) = app.dsl_view.pack_root.clone() {
        let pack_source = pack_source_from_root(&pack_root)?;
        DeployDslRequest {
            source: String::new(),
            name: config_name,
            activate: true,
            notes: Some("Deployed from TUI pack".to_string()),
            pack: Some(pack_source),
        }
    } else {
        let source = app.dsl_view.content.clone();
        if source.trim().is_empty() {
            app.notify(
                caliber_tui::notifications::NotificationLevel::Warning,
                "DSL content is empty; nothing to deploy.",
            );
            return Ok(());
        }
        DeployDslRequest {
            source,
            name: config_name,
            activate: true,
            notes: Some("Deployed from TUI DSL editor".to_string()),
            pack: None,
        }
    };

    let response = app.api.rest().deploy_dsl(tenant_id, &request).await?;
    app.notify(
        caliber_tui::notifications::NotificationLevel::Success,
        response.message,
    );
    Ok(())
}

fn dsl_from_ast_value(ast_value: &serde_json::Value) -> Option<String> {
    serde_json::from_value::<CaliberAst>(ast_value.clone())
        .ok()
        .map(|ast| pretty_print(&ast))
}

fn pack_source_from_root(root: &Path) -> Result<PackSource, TuiError> {
    let manifest_path = root.join("cal.toml");
    let manifest = fs::read_to_string(&manifest_path)?;
    let markdowns = collect_markdowns(root)?;
    Ok(PackSource {
        manifest,
        markdowns,
    })
}

fn collect_markdowns(root: &Path) -> Result<Vec<PackSourceFile>, TuiError> {
    let mut out = Vec::new();
    collect_markdowns_recursive(root, root, &mut out)?;
    Ok(out)
}

fn collect_markdowns_recursive(
    root: &Path,
    dir: &Path,
    out: &mut Vec<PackSourceFile>,
) -> Result<(), TuiError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            collect_markdowns_recursive(root, &path, out)?;
            continue;
        }

        let is_markdown = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("md"))
            .unwrap_or(false);

        if !is_markdown {
            continue;
        }

        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        let content = fs::read_to_string(&path)?;
        out.push(PackSourceFile {
            path: rel_path,
            content,
        });
    }
    Ok(())
}
