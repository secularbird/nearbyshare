use crate::state::{AppState, Peer};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

const SERVICE_TYPE: &str = "_nearbyshare._tcp.local.";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPeer {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub port: u16,
}

/// Start discovering peers on the local network
#[tauri::command]
pub async fn start_discovery(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let mut running = state.discovery_running.write().map_err(|e| e.to_string())?;
    if *running {
        return Ok(());
    }
    *running = true;
    drop(running);

    let state_clone = Arc::clone(&state);
    let app_clone = app.clone();
    
    std::thread::spawn(move || {
        let mdns = match ServiceDaemon::new() {
            Ok(m) => m,
            Err(e) => {
                log::error!("Failed to create mDNS daemon: {}", e);
                return;
            }
        };
        
        let receiver = match mdns.browse(SERVICE_TYPE) {
            Ok(r) => r,
            Err(e) => {
                log::error!("Failed to browse mDNS services: {}", e);
                return;
            }
        };

        let device_id = state_clone.device_id.clone();
        
        loop {
            // Check if discovery should stop
            if let Ok(running) = state_clone.discovery_running.read() {
                if !*running {
                    break;
                }
            }
            
            match receiver.recv_timeout(Duration::from_millis(500)) {
                Ok(event) => {
                    match event {
                        ServiceEvent::ServiceResolved(info) => {
                            // Skip our own service
                            if let Some(id) = info.get_property_val_str("id") {
                                if id == device_id {
                                    continue;
                                }
                            }
                            
                            if let Some(addr) = info.get_addresses().iter().next() {
                                let peer_id = info.get_property_val_str("id")
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| info.get_fullname().to_string());
                                let peer_name = info.get_property_val_str("name")
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| info.get_hostname().to_string());
                                
                                let peer = Peer {
                                    id: peer_id.clone(),
                                    name: peer_name.to_string(),
                                    ip: addr.to_string(),
                                    port: info.get_port(),
                                    connected: false,
                                    authenticated: false,
                                };
                                
                                state_clone.add_discovered_peer(peer.clone());
                                
                                // Emit event to frontend
                                let _ = app_clone.emit("peer-discovered", DiscoveredPeer {
                                    id: peer.id,
                                    name: peer.name,
                                    ip: peer.ip,
                                    port: peer.port,
                                });
                            }
                        }
                        ServiceEvent::ServiceRemoved(_, fullname) => {
                            // Extract peer ID from fullname and remove
                            let peer_id = fullname.split('.').next().unwrap_or(&fullname).to_string();
                            state_clone.remove_discovered_peer(&peer_id);
                            
                            // Emit event to frontend
                            let _ = app_clone.emit("peer-removed", peer_id);
                        }
                        _ => {}
                    }
                }
                Err(_) => {
                    // Timeout or disconnected, continue checking if we should stop
                    continue;
                }
            }
        }
    });
    
    Ok(())
}

/// Stop discovering peers
#[tauri::command]
pub fn stop_discovery(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut running = state.discovery_running.write().map_err(|e| e.to_string())?;
    *running = false;
    Ok(())
}

/// Get list of discovered peers
#[tauri::command]
pub fn get_peers(state: State<'_, Arc<AppState>>) -> Result<Vec<Peer>, String> {
    let peers = state.discovered_peers.read().map_err(|e| e.to_string())?;
    Ok(peers.values().cloned().collect())
}

/// Register this device as a discoverable service
#[tauri::command]
pub fn register_service(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let device_id = state.device_id.clone();
    let device_name = state.device_name.read().map_err(|e| e.to_string())?.clone();
    let port = *state.server_port.read().map_err(|e| e.to_string())?;
    
    std::thread::spawn(move || {
        let mdns = match ServiceDaemon::new() {
            Ok(m) => m,
            Err(e) => {
                log::error!("Failed to create mDNS daemon: {}", e);
                return;
            }
        };
        
        let service_name = format!("{}._nearbyshare._tcp.local.", device_id);
        let host_name = format!("{}.local.", device_name);
        
        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            &device_id,
            &host_name,
            "",
            port,
            &[("id", &device_id), ("name", &device_name)][..],
        );
        
        match service_info {
            Ok(info) => {
                if let Err(e) = mdns.register(info) {
                    log::error!("Failed to register mDNS service: {}", e);
                } else {
                    log::info!("Registered mDNS service: {}", service_name);
                }
            }
            Err(e) => {
                log::error!("Failed to create service info: {}", e);
            }
        }
    });
    
    Ok(())
}

/// Unregister this device's service
#[tauri::command]
pub fn unregister_service(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let device_id = state.device_id.clone();
    
    std::thread::spawn(move || {
        let mdns = match ServiceDaemon::new() {
            Ok(m) => m,
            Err(e) => {
                log::error!("Failed to create mDNS daemon: {}", e);
                return;
            }
        };
        
        let service_name = format!("{}.{}", device_id, SERVICE_TYPE);
        if let Err(e) = mdns.unregister(&service_name) {
            log::error!("Failed to unregister mDNS service: {}", e);
        }
    });
    
    Ok(())
}
