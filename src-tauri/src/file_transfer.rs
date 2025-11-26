use crate::network::{send_message_to_peer, Message};
use crate::state::{AppState, FileTransfer, TransferStatus};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

const CHUNK_SIZE: usize = 65536; // 64KB chunks

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgress {
    pub id: String,
    pub file_name: String,
    pub file_size: u64,
    pub transferred: u64,
    pub status: TransferStatus,
}

/// Send a file to a connected peer
#[tauri::command]
pub async fn send_file(
    peer_id: String,
    file_path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    // Get peer info
    let peers = state.connected_peers.read().map_err(|e| e.to_string())?;
    let peer = peers.get(&peer_id).ok_or("Peer not found")?.clone();
    drop(peers);
    
    // Get file info
    let file = File::open(&file_path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    let file_size = metadata.len();
    let file_name = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    // Create transfer record
    let transfer_id = Uuid::new_v4().to_string();
    let transfer = FileTransfer {
        id: transfer_id.clone(),
        file_name: file_name.clone(),
        file_size,
        transferred: 0,
        from_peer: "self".to_string(),
        to_peer: peer_id.clone(),
        status: TransferStatus::Pending,
        file_path: Some(file_path.clone()),
    };
    state.add_transfer(transfer);
    
    // Send file request
    let request = Message::FileRequest {
        id: transfer_id.clone(),
        name: file_name,
        size: file_size,
    };
    send_message_to_peer(&peer, &request)?;
    
    Ok(transfer_id)
}

/// Get transfer progress
#[tauri::command]
pub fn get_transfer_progress(
    transfer_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<TransferProgress>, String> {
    let transfers = state.transfers.read().map_err(|e| e.to_string())?;
    
    Ok(transfers.get(&transfer_id).map(|t| TransferProgress {
        id: t.id.clone(),
        file_name: t.file_name.clone(),
        file_size: t.file_size,
        transferred: t.transferred,
        status: t.status.clone(),
    }))
}

/// Cancel a file transfer
#[tauri::command]
pub fn cancel_transfer(
    transfer_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.update_transfer_status(&transfer_id, TransferStatus::Cancelled);
    Ok(())
}

/// Get all pending incoming transfers
#[tauri::command]
pub fn get_pending_transfers(state: State<'_, Arc<AppState>>) -> Result<Vec<FileTransfer>, String> {
    let transfers = state.transfers.read().map_err(|e| e.to_string())?;
    
    Ok(transfers
        .values()
        .filter(|t| t.status == TransferStatus::Pending)
        .cloned()
        .collect())
}

/// Accept an incoming file transfer
#[tauri::command]
pub async fn accept_transfer(
    transfer_id: String,
    save_path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    // Update transfer with save path
    {
        let mut transfers = state.transfers.write().map_err(|e| e.to_string())?;
        if let Some(transfer) = transfers.get_mut(&transfer_id) {
            transfer.file_path = Some(save_path);
            transfer.status = TransferStatus::Accepted;
        }
    }
    
    // Get transfer info
    let transfers = state.transfers.read().map_err(|e| e.to_string())?;
    let transfer = transfers.get(&transfer_id).ok_or("Transfer not found")?.clone();
    drop(transfers);
    
    // Get peer info
    let peers = state.connected_peers.read().map_err(|e| e.to_string())?;
    let peer = peers.get(&transfer.from_peer).ok_or("Peer not found")?.clone();
    drop(peers);
    
    // Send accept message
    let accept = Message::FileAccept { id: transfer_id };
    send_message_to_peer(&peer, &accept)?;
    
    Ok(())
}

/// Reject an incoming file transfer
#[tauri::command]
pub async fn reject_transfer(
    transfer_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.update_transfer_status(&transfer_id, TransferStatus::Rejected);
    
    // Get transfer info
    let transfers = state.transfers.read().map_err(|e| e.to_string())?;
    let transfer = transfers.get(&transfer_id).ok_or("Transfer not found")?.clone();
    drop(transfers);
    
    // Get peer info
    let peers = state.connected_peers.read().map_err(|e| e.to_string())?;
    let peer = peers.get(&transfer.from_peer).ok_or("Peer not found")?.clone();
    drop(peers);
    
    // Send reject message
    let reject = Message::FileReject { id: transfer_id };
    send_message_to_peer(&peer, &reject)?;
    
    Ok(())
}

/// Internal function to send file data chunks
pub fn send_file_chunks(
    peer_id: &str,
    transfer_id: &str,
    file_path: &str,
    state: &Arc<AppState>,
) -> Result<(), String> {
    let peers = state.connected_peers.read().map_err(|e| e.to_string())?;
    let peer = peers.get(peer_id).ok_or("Peer not found")?.clone();
    drop(peers);
    
    let mut file = File::open(file_path).map_err(|e| e.to_string())?;
    let file_size = file.metadata().map_err(|e| e.to_string())?.len();
    
    let mut offset: u64 = 0;
    let mut buffer = vec![0u8; CHUNK_SIZE];
    
    while offset < file_size {
        // Check if transfer was cancelled
        let transfers = state.transfers.read().map_err(|e| e.to_string())?;
        if let Some(transfer) = transfers.get(transfer_id) {
            if transfer.status == TransferStatus::Cancelled {
                return Err("Transfer cancelled".to_string());
            }
        }
        drop(transfers);
        
        file.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
        let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        
        if bytes_read == 0 {
            break;
        }
        
        let data = BASE64.encode(&buffer[..bytes_read]);
        let message = Message::FileData {
            id: transfer_id.to_string(),
            data,
            offset,
        };
        
        send_message_to_peer(&peer, &message)?;
        
        offset += bytes_read as u64;
        state.update_transfer_progress(transfer_id, offset);
    }
    
    // Send completion message
    let complete = Message::FileComplete {
        id: transfer_id.to_string(),
    };
    send_message_to_peer(&peer, &complete)?;
    state.update_transfer_status(transfer_id, TransferStatus::Completed);
    
    Ok(())
}
