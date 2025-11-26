mod network;
mod discovery;
mod clipboard;
mod file_transfer;
mod auth;
mod state;

use state::AppState;
use std::sync::Arc;
use tauri::Manager;

// Re-export commands
pub use network::*;
pub use discovery::*;
pub use clipboard::*;
pub use file_transfer::*;
pub use auth::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_state = Arc::new(AppState::new());
            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Auth commands
            auth::generate_token,
            auth::set_token,
            auth::get_token,
            auth::verify_token,
            // Discovery commands
            discovery::start_discovery,
            discovery::stop_discovery,
            discovery::get_peers,
            discovery::register_service,
            discovery::unregister_service,
            // Network commands
            network::connect_to_peer,
            network::disconnect_from_peer,
            network::get_connected_peers,
            network::start_server,
            network::stop_server,
            // Clipboard commands
            clipboard::share_clipboard,
            clipboard::get_shared_clipboard,
            clipboard::enable_clipboard_sync,
            clipboard::disable_clipboard_sync,
            // File transfer commands
            file_transfer::send_file,
            file_transfer::get_transfer_progress,
            file_transfer::cancel_transfer,
            file_transfer::get_pending_transfers,
            file_transfer::accept_transfer,
            file_transfer::reject_transfer,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
