#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! FluxAiBot — main entry point for the Tauri desktop shell.

use std::sync::Arc;
use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem},
    Manager,
};

#[tauri::command]
async fn get_status(
    state: tauri::State<'_, Arc<local_ai_hub_lib::app_state::AppState>>,
) -> Result<serde_json::Value, String> {
    state.get_status().await
}

#[tauri::command]
async fn get_api_token(
    state: tauri::State<'_, Arc<local_ai_hub_lib::app_state::AppState>>,
) -> Result<String, String> {
    Ok(state.api_token().to_string())
}

#[tauri::command]
async fn discover_integrations(
    state: tauri::State<'_, Arc<local_ai_hub_lib::app_state::AppState>>,
) -> Result<serde_json::Value, String> {
    state.discover_integrations().await
}

#[tauri::command]
async fn get_capabilities(
    state: tauri::State<'_, Arc<local_ai_hub_lib::app_state::AppState>>,
) -> Result<serde_json::Value, String> {
    state.get_capabilities().await
}

#[tauri::command]
async fn start_runtime(
    state: tauri::State<'_, Arc<local_ai_hub_lib::app_state::AppState>>,
    model_id: String,
) -> Result<serde_json::Value, String> {
    state.start_runtime(&model_id).await
}

#[tauri::command]
async fn stop_runtime(
    state: tauri::State<'_, Arc<local_ai_hub_lib::app_state::AppState>>,
) -> Result<serde_json::Value, String> {
    state.stop_runtime().await
}

#[tauri::command]
async fn scan_models(
    state: tauri::State<'_, Arc<local_ai_hub_lib::app_state::AppState>>,
    folders: Vec<String>,
) -> Result<serde_json::Value, String> {
    state.scan_models(&folders).await
}

#[tauri::command]
async fn search_huggingface_models(
    state: tauri::State<'_, Arc<local_ai_hub_lib::app_state::AppState>>,
    query: String,
    max_size_gb: Option<u64>,
) -> Result<local_ai_hub_lib::model_hub::HubSearchResult, String> {
    state.search_huggingface_models(&query, max_size_gb).await
}

#[tauri::command]
async fn start_huggingface_download(
    state: tauri::State<'_, Arc<local_ai_hub_lib::app_state::AppState>>,
    repo_id: String,
    revision: String,
    files: Vec<String>,
) -> Result<(), String> {
    state
        .inner()
        .start_huggingface_download(repo_id, revision, files)
        .await
}

#[tauri::command]
async fn get_huggingface_download_progress(
    state: tauri::State<'_, Arc<local_ai_hub_lib::app_state::AppState>>,
) -> Result<local_ai_hub_lib::model_hub::DownloadProgress, String> {
    Ok(state.get_huggingface_download_progress().await)
}

#[tauri::command]
async fn cancel_huggingface_download(
    state: tauri::State<'_, Arc<local_ai_hub_lib::app_state::AppState>>,
) -> Result<(), String> {
    state.cancel_huggingface_download().await
}

#[tauri::command]
async fn toggle_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    let window = app_handle
        .get_webview_window("main")
        .ok_or_else(|| "Main window is unavailable".to_string())?;
    if window
        .is_visible()
        .map_err(|e| format!("Failed to inspect main window: {e}"))?
    {
        window
            .hide()
            .map_err(|e| format!("Failed to hide main window: {e}"))
    } else {
        window
            .show()
            .and_then(|_| window.set_focus())
            .map_err(|e| format!("Failed to show main window: {e}"))
    }
}

#[tauri::command]
async fn open_folder_dialog(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_folder(move |folder| {
        let _ = sender.send(folder.map(|path| path.to_string()));
    });
    receiver
        .await
        .map_err(|e| format!("Folder picker did not return a result: {e}"))
}

fn main() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            let state = tauri::async_runtime::block_on(async {
                local_ai_hub_lib::app_state::AppState::new().await
            })
            .map_err(|e| format!("Failed to initialize app state: {e}"))?;
            app.manage(state.clone());

            let gateway_router = local_ai_hub_lib::gateway::build_router(Arc::new(
                local_ai_hub_lib::gateway::GatewayState::from_app(&state),
            ));
            match std::net::TcpListener::bind(("127.0.0.1", state.gateway_port)) {
                Ok(listener) => {
                    listener
                        .set_nonblocking(true)
                        .map_err(|e| format!("Failed to configure local API listener: {e}"))?;
                    let (ready_sender, ready_receiver) =
                        std::sync::mpsc::sync_channel::<Result<(), String>>(1);
                    let gateway_app = state.clone();
                    let spawn_result = std::thread::Builder::new()
                        .name("fluxaibot-gateway".to_string())
                        .spawn(move || {
                            let runtime = match tokio::runtime::Builder::new_multi_thread()
                                .enable_all()
                                .build()
                            {
                                Ok(runtime) => runtime,
                                Err(error) => {
                                    let _ = ready_sender.send(Err(format!(
                                        "Failed to create gateway runtime: {error}"
                                    )));
                                    return;
                                }
                            };
                            let listener = match runtime
                                .block_on(async { tokio::net::TcpListener::from_std(listener) })
                            {
                                Ok(listener) => listener,
                                Err(error) => {
                                    let _ = ready_sender.send(Err(format!(
                                        "Failed to start local API listener: {error}"
                                    )));
                                    return;
                                }
                            };
                            let _ = ready_sender.send(Ok(()));
                            runtime.block_on(async move {
                                if let Err(error) = axum::serve(listener, gateway_router).await {
                                    eprintln!("Local API gateway stopped: {error}");
                                    gateway_app
                                        .set_gateway_status(
                                            false,
                                            Some(
                                                "The local API gateway stopped unexpectedly"
                                                    .to_string(),
                                            ),
                                        )
                                        .await;
                                }
                            });
                        });

                    if let Err(error) = spawn_result {
                        tauri::async_runtime::block_on(state.set_gateway_status(
                            false,
                            Some(format!("Failed to start local API gateway: {error}")),
                        ));
                    } else {
                        match ready_receiver.recv_timeout(Duration::from_secs(5)) {
                            Ok(Ok(())) => {
                                tauri::async_runtime::block_on(state.set_gateway_status(true, None))
                            }
                            Ok(Err(error)) => tauri::async_runtime::block_on(
                                state.set_gateway_status(false, Some(error)),
                            ),
                            Err(error) => tauri::async_runtime::block_on(state.set_gateway_status(
                                false,
                                Some(format!("Local API gateway startup timed out: {error}")),
                            )),
                        }
                    }
                }
                Err(error) => {
                    tauri::async_runtime::block_on(state.set_gateway_status(
                        false,
                        Some(format!(
                            "Cannot bind the local API gateway to 127.0.0.1:{}: {error}",
                            state.gateway_port
                        )),
                    ));
                }
            }

            let tauri_url = tauri::WebviewUrl::App("index.html".into());
            let window = tauri::WebviewWindowBuilder::new(app, "main", tauri_url)
                .title("FluxAiBot")
                .inner_size(1280.0, 860.0)
                .min_inner_size(960.0, 640.0)
                .resizable(true)
                .decorations(true)
                .visible(true)
                .build()
                .map_err(|e| format!("Failed to create main window: {e}"))?;
            app.manage(window);

            let show_item = MenuItem::new(app, "show", true, Option::<&str>::None)?;
            let quit_item = MenuItem::new(app, "quit", true, Option::<&str>::None)?;
            let tray_menu = Menu::new(app.handle())?;
            tray_menu.append_items(&[&show_item, &quit_item])?;
            let icon = app
                .default_window_icon()
                .ok_or_else(|| "Default application icon is unavailable".to_string())?
                .clone();
            let _tray = tauri::tray::TrayIconBuilder::new()
                .icon(icon)
                .menu(&tray_menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            let supervisor = state.clone();
            tauri::async_runtime::spawn(async move {
                supervisor.run_supervisor().await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_api_token,
            discover_integrations,
            get_capabilities,
            start_runtime,
            stop_runtime,
            scan_models,
            search_huggingface_models,
            start_huggingface_download,
            get_huggingface_download_progress,
            cancel_huggingface_download,
            toggle_window,
            open_folder_dialog,
        ]);

    let application = builder
        .build(tauri::generate_context!())
        .unwrap_or_else(|error| {
            eprintln!("FluxAiBot failed to initialize: {error}");
            std::process::exit(1);
        });
    application.run(|app_handle, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            let state = app_handle.state::<Arc<local_ai_hub_lib::app_state::AppState>>();
            tauri::async_runtime::block_on(state.shutdown());
        }
    });
}
