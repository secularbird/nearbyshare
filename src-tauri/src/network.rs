use crate::auth::hash_token;
use crate::state::{AppState, FileTransfer, Peer, TransferStatus};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// Maximum allowed message size (10 MB)
const MAX_MESSAGE_SIZE_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    Auth { token_hash: String },
    AuthResponse { success: bool },
    Clipboard { content: String },
    FileRequest { id: String, name: String, size: u64 },
    FileAccept { id: String },
    FileReject { id: String },
    FileData { id: String, data: String, offset: u64 },
    FileComplete { id: String },
    Ping,
    Pong,
}

/// Connect to a peer with authentication
#[tauri::command]
pub async fn connect_to_peer(
    peer_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<bool, String> {
    let peers = state.discovered_peers.read().map_err(|e| e.to_string())?;
    let peer = peers.get(&peer_id).ok_or("Peer not found")?.clone();
    drop(peers);
    
    let token = state.token.read().map_err(|e| e.to_string())?;
    let token_hash = match &*token {
        Some(t) => hash_token(t),
        None => return Err("No token set".to_string()),
    };
    drop(token);
    
    let addr = format!("{}:{}", peer.ip, peer.port);
    let mut stream = TcpStream::connect(&addr).map_err(|e| e.to_string())?;
    
    // Send authentication message
    let auth_msg = Message::Auth { token_hash };
    let msg_bytes = serde_json::to_vec(&auth_msg).map_err(|e| e.to_string())?;
    let len_bytes = (msg_bytes.len() as u32).to_be_bytes();
    stream.write_all(&len_bytes).map_err(|e| e.to_string())?;
    stream.write_all(&msg_bytes).map_err(|e| e.to_string())?;
    
    // Read authentication response
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
    let msg_len = u32::from_be_bytes(len_buf) as usize;
    
    let mut msg_buf = vec![0u8; msg_len];
    stream.read_exact(&mut msg_buf).map_err(|e| e.to_string())?;
    
    let response: Message = serde_json::from_slice(&msg_buf).map_err(|e| e.to_string())?;
    
    match response {
        Message::AuthResponse { success } => {
            if success {
                let mut connected_peer = peer.clone();
                connected_peer.connected = true;
                connected_peer.authenticated = true;
                state.add_connected_peer(connected_peer);
                Ok(true)
            } else {
                Ok(false)
            }
        }
        _ => Err("Unexpected response".to_string()),
    }
}

/// Disconnect from a peer
#[tauri::command]
pub fn disconnect_from_peer(
    peer_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.remove_connected_peer(&peer_id);
    Ok(())
}

/// Get list of connected peers
#[tauri::command]
pub fn get_connected_peers(state: State<'_, Arc<AppState>>) -> Result<Vec<Peer>, String> {
    let peers = state.connected_peers.read().map_err(|e| e.to_string())?;
    Ok(peers.values().cloned().collect())
}

/// Start the server to accept incoming connections
#[tauri::command]
pub async fn start_server(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<u16, String> {
    let mut running = state.server_running.write().map_err(|e| e.to_string())?;
    if *running {
        return Ok(*state.server_port.read().map_err(|e| e.to_string())?);
    }
    
    // Find an available port
    let listener = TcpListener::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    
    *state.server_port.write().map_err(|e| e.to_string())? = port;
    *running = true;
    drop(running);
    
    let state_clone = Arc::clone(&state);
    let app_clone = app.clone();
    
    std::thread::spawn(move || {
        log::info!("Server listening on port {}", port);
        
        for stream in listener.incoming() {
            // Check if server should stop
            if let Ok(running) = state_clone.server_running.read() {
                if !*running {
                    break;
                }
            }
            
            match stream {
                Ok(stream) => {
                    let state_for_client = Arc::clone(&state_clone);
                    let app_for_client = app_clone.clone();
                    
                    std::thread::spawn(move || {
                        if let Err(e) = handle_client(stream, state_for_client, app_for_client) {
                            log::error!("Client error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    log::error!("Failed to accept connection: {}", e);
                }
            }
        }
    });
    
    Ok(port)
}

fn handle_client(
    mut stream: TcpStream,
    state: Arc<AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let peer_addr = stream.peer_addr().map_err(|e| e.to_string())?;
    log::info!("New connection from {}", peer_addr);
    
    // Read authentication message
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
    let msg_len = u32::from_be_bytes(len_buf) as usize;
    
    let mut msg_buf = vec![0u8; msg_len];
    stream.read_exact(&mut msg_buf).map_err(|e| e.to_string())?;
    
    let message: Message = serde_json::from_slice(&msg_buf).map_err(|e| e.to_string())?;
    
    match message {
        Message::Auth { token_hash } => {
            let stored_token = state.token.read().map_err(|e| e.to_string())?;
            let success = match &*stored_token {
                Some(t) => hash_token(t) == token_hash,
                None => false,
            };
            drop(stored_token);
            
            let response = Message::AuthResponse { success };
            let msg_bytes = serde_json::to_vec(&response).map_err(|e| e.to_string())?;
            let len_bytes = (msg_bytes.len() as u32).to_be_bytes();
            stream.write_all(&len_bytes).map_err(|e| e.to_string())?;
            stream.write_all(&msg_bytes).map_err(|e| e.to_string())?;
            
            if success {
                let peer = Peer {
                    id: peer_addr.to_string(),
                    name: peer_addr.ip().to_string(),
                    ip: peer_addr.ip().to_string(),
                    port: peer_addr.port(),
                    connected: true,
                    authenticated: true,
                };
                state.add_connected_peer(peer.clone());
                
                // Emit event to frontend
                let _ = app.emit("peer-connected", peer);
                
                // Handle subsequent messages
                handle_authenticated_client(stream, state, app)?;
            }
        }
        _ => {
            return Err(format!("Expected authentication message, but received a different message type from {}", peer_addr));
        }
    }
    
    Ok(())
}

fn handle_authenticated_client(
    mut stream: TcpStream,
    state: Arc<AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let peer_addr = stream.peer_addr().map_err(|e| e.to_string())?;
    
    loop {
        let mut len_buf = [0u8; 4];
        if stream.read_exact(&mut len_buf).is_err() {
            // Connection closed
            state.remove_connected_peer(&peer_addr.to_string());
            let _ = app.emit("peer-disconnected", peer_addr.to_string());
            break;
        }
        
        let msg_len = u32::from_be_bytes(len_buf) as usize;
        if msg_len > MAX_MESSAGE_SIZE_BYTES {
            // Message too large, skip
            continue;
        }
        
        let mut msg_buf = vec![0u8; msg_len];
        if stream.read_exact(&mut msg_buf).is_err() {
            break;
        }
        
        let message: Message = match serde_json::from_slice(&msg_buf) {
            Ok(m) => m,
            Err(_) => continue,
        };
        
        match message {
            Message::Clipboard { content } => {
                *state.shared_clipboard.write().map_err(|e| e.to_string())? = Some(content.clone());
                let _ = app.emit("clipboard-received", content);
            }
            Message::FileRequest { id, name, size } => {
                let transfer = FileTransfer {
                    id: id.clone(),
                    file_name: name.clone(),
                    file_size: size,
                    transferred: 0,
                    from_peer: peer_addr.to_string(),
                    to_peer: "self".to_string(),
                    status: TransferStatus::Pending,
                    file_path: None,
                };
                state.add_transfer(transfer.clone());
                let _ = app.emit("file-transfer-request", transfer);
            }
            Message::FileAccept { id } => {
                state.update_transfer_status(&id, TransferStatus::Accepted);
                let _ = app.emit("file-transfer-accepted", id);
            }
            Message::FileReject { id } => {
                state.update_transfer_status(&id, TransferStatus::Rejected);
                let _ = app.emit("file-transfer-rejected", id);
            }
            Message::FileData { id, data, offset } => {
                state.update_transfer_progress(&id, offset);
                state.update_transfer_status(&id, TransferStatus::InProgress);
                let _ = app.emit("file-transfer-progress", serde_json::json!({
                    "id": id,
                    "data": data,
                    "offset": offset
                }));
            }
            Message::FileComplete { id } => {
                state.update_transfer_status(&id, TransferStatus::Completed);
                let _ = app.emit("file-transfer-complete", id);
            }
            Message::Ping => {
                let response = Message::Pong;
                let msg_bytes = serde_json::to_vec(&response).unwrap_or_default();
                let len_bytes = (msg_bytes.len() as u32).to_be_bytes();
                let _ = stream.write_all(&len_bytes);
                let _ = stream.write_all(&msg_bytes);
            }
            _ => {}
        }
    }
    
    Ok(())
}

/// Stop the server
#[tauri::command]
pub fn stop_server(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut running = state.server_running.write().map_err(|e| e.to_string())?;
    *running = false;
    Ok(())
}

/// Send a message to a connected peer
pub fn send_message_to_peer(peer: &Peer, message: &Message) -> Result<(), String> {
    let addr = format!("{}:{}", peer.ip, peer.port);
    let mut stream = TcpStream::connect(&addr).map_err(|e| e.to_string())?;
    
    let msg_bytes = serde_json::to_vec(message).map_err(|e| e.to_string())?;
    let len_bytes = (msg_bytes.len() as u32).to_be_bytes();
    stream.write_all(&len_bytes).map_err(|e| e.to_string())?;
    stream.write_all(&msg_bytes).map_err(|e| e.to_string())?;
    
    Ok(())
}
