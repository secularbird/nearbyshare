use crate::state::AppState;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

/// Generate a new random token for authentication
#[tauri::command]
pub fn generate_token(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let token = Uuid::new_v4().to_string();
    let mut stored_token = state.token.write().map_err(|e| e.to_string())?;
    *stored_token = Some(token.clone());
    Ok(token)
}

/// Set a custom token for authentication
#[tauri::command]
pub fn set_token(token: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    if token.is_empty() {
        return Err("Token cannot be empty".to_string());
    }
    let mut stored_token = state.token.write().map_err(|e| e.to_string())?;
    *stored_token = Some(token);
    Ok(())
}

/// Get the current token
#[tauri::command]
pub fn get_token(state: State<'_, Arc<AppState>>) -> Result<Option<String>, String> {
    let token = state.token.read().map_err(|e| e.to_string())?;
    Ok(token.clone())
}

/// Verify if a given token matches the stored token
#[tauri::command]
pub fn verify_token(token: String, state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    let stored_token = state.token.read().map_err(|e| e.to_string())?;
    match &*stored_token {
        Some(stored) => Ok(hash_token(&token) == hash_token(stored)),
        None => Ok(false),
    }
}

/// Hash a token for secure comparison
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}
