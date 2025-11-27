use crate::network::{send_message_to_peer, Message};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

/// Share clipboard content with all connected peers
#[tauri::command]
pub fn share_clipboard(
    content: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    // Store locally
    *state.shared_clipboard.write().map_err(|e| e.to_string())? = Some(content.clone());
    
    // Send to all connected peers
    let peers = state.connected_peers.read().map_err(|e| e.to_string())?;
    let message = Message::Clipboard { content };
    
    for peer in peers.values() {
        if let Err(e) = send_message_to_peer(peer, &message) {
            log::error!("Failed to send clipboard to peer {}: {}", peer.id, e);
        }
    }
    
    Ok(())
}

/// Get the shared clipboard content
#[tauri::command]
pub fn get_shared_clipboard(state: State<'_, Arc<AppState>>) -> Result<Option<String>, String> {
    let clipboard = state.shared_clipboard.read().map_err(|e| e.to_string())?;
    Ok(clipboard.clone())
}

/// Enable automatic clipboard synchronization
#[tauri::command]
pub fn enable_clipboard_sync(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut sync_enabled = state.clipboard_sync_enabled.write().map_err(|e| e.to_string())?;
    *sync_enabled = true;
    Ok(())
}

/// Disable automatic clipboard synchronization
#[tauri::command]
pub fn disable_clipboard_sync(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut sync_enabled = state.clipboard_sync_enabled.write().map_err(|e| e.to_string())?;
    *sync_enabled = false;
    Ok(())
}
