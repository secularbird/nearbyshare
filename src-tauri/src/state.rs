use std::collections::HashMap;
use std::sync::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub connected: bool,
    pub authenticated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransfer {
    pub id: String,
    pub file_name: String,
    pub file_size: u64,
    pub transferred: u64,
    pub from_peer: String,
    pub to_peer: String,
    pub status: TransferStatus,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransferStatus {
    Pending,
    Accepted,
    InProgress,
    Completed,
    Failed,
    Cancelled,
    Rejected,
}

pub struct AppState {
    pub token: RwLock<Option<String>>,
    pub device_id: String,
    pub device_name: RwLock<String>,
    pub discovered_peers: RwLock<HashMap<String, Peer>>,
    pub connected_peers: RwLock<HashMap<String, Peer>>,
    pub shared_clipboard: RwLock<Option<String>>,
    pub clipboard_sync_enabled: RwLock<bool>,
    pub transfers: RwLock<HashMap<String, FileTransfer>>,
    pub server_running: RwLock<bool>,
    pub server_port: RwLock<u16>,
    pub discovery_running: RwLock<bool>,
}

impl AppState {
    pub fn new() -> Self {
        let device_id = Uuid::new_v4().to_string();
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "Unknown".to_string());
        
        Self {
            token: RwLock::new(None),
            device_id,
            device_name: RwLock::new(hostname),
            discovered_peers: RwLock::new(HashMap::new()),
            connected_peers: RwLock::new(HashMap::new()),
            shared_clipboard: RwLock::new(None),
            clipboard_sync_enabled: RwLock::new(false),
            transfers: RwLock::new(HashMap::new()),
            server_running: RwLock::new(false),
            server_port: RwLock::new(8765),
            discovery_running: RwLock::new(false),
        }
    }
    
    pub fn add_discovered_peer(&self, peer: Peer) {
        let mut peers = self.discovered_peers.write().unwrap();
        peers.insert(peer.id.clone(), peer);
    }
    
    pub fn remove_discovered_peer(&self, peer_id: &str) {
        let mut peers = self.discovered_peers.write().unwrap();
        peers.remove(peer_id);
    }
    
    pub fn add_connected_peer(&self, peer: Peer) {
        let mut peers = self.connected_peers.write().unwrap();
        peers.insert(peer.id.clone(), peer);
    }
    
    pub fn remove_connected_peer(&self, peer_id: &str) {
        let mut peers = self.connected_peers.write().unwrap();
        peers.remove(peer_id);
    }
    
    pub fn add_transfer(&self, transfer: FileTransfer) {
        let mut transfers = self.transfers.write().unwrap();
        transfers.insert(transfer.id.clone(), transfer);
    }
    
    pub fn update_transfer_status(&self, transfer_id: &str, status: TransferStatus) {
        let mut transfers = self.transfers.write().unwrap();
        if let Some(transfer) = transfers.get_mut(transfer_id) {
            transfer.status = status;
        }
    }
    
    pub fn update_transfer_progress(&self, transfer_id: &str, transferred: u64) {
        let mut transfers = self.transfers.write().unwrap();
        if let Some(transfer) = transfers.get_mut(transfer_id) {
            transfer.transferred = transferred;
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
