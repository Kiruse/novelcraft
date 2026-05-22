use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tokio::sync::{Mutex, OnceCell};

use crate::error::AppError;
use crate::util;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Profile {
  pub id: String,
  pub name: String,
  pub fields: HashMap<String, String>,
  pub created_at: String,
  pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ProfileListResult {
  pub profiles: Vec<Profile>,
  pub active_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProfilesFile {
  version: u32,
  profiles: Vec<Profile>,
  active_id: Option<String>,
}

static PROFILES: OnceCell<Mutex<ProfilesFile>> = OnceCell::const_new();

fn profiles_path(app: &AppHandle) -> Result<PathBuf, AppError> {
  Ok(
    app
      .path()
      .app_data_dir()
      .map_err(|e| AppError::internal(e.to_string()))?
      .join("profiles.json"),
  )
}

async fn save_to_disk(app: &AppHandle, file: &ProfilesFile) -> Result<(), AppError> {
  let path = profiles_path(app)?;
  util::serialize(&path, file).await
}

pub async fn init_profiles(app: &AppHandle) {
  let path = profiles_path(app).unwrap();
  let file = if path.exists() {
    let content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
    serde_json::from_str(&content).unwrap_or(ProfilesFile {
      version: 1,
      profiles: Vec::new(),
      active_id: None,
    })
  } else {
    ProfilesFile {
      version: 1,
      profiles: Vec::new(),
      active_id: None,
    }
  };
  PROFILES.set(Mutex::new(file)).unwrap();
}

#[tauri::command]
#[specta::specta]
pub async fn profile_list() -> Result<ProfileListResult, AppError> {
  let store = PROFILES.get().ok_or(AppError::internal("Profiles not initialized"))?;
  let guard = store.lock().await;
  Ok(ProfileListResult {
    profiles: guard.profiles.clone(),
    active_id: guard.active_id.clone(),
  })
}

#[tauri::command]
#[specta::specta]
pub async fn profile_create(
  app: AppHandle,
  id: String,
  name: String,
  fields: HashMap<String, String>,
  created_at: String,
) -> Result<(), AppError> {
  let store = PROFILES.get().ok_or(AppError::internal("Profiles not initialized"))?;
  let mut guard = store.lock().await;
  guard.profiles.push(Profile {
    id,
    name,
    fields,
    created_at: created_at.clone(),
    updated_at: created_at,
  });
  save_to_disk(&app, &guard).await
}

#[tauri::command]
#[specta::specta]
pub async fn profile_update(
  app: AppHandle,
  id: String,
  name: String,
  fields: HashMap<String, String>,
  updated_at: String,
) -> Result<(), AppError> {
  let store = PROFILES.get().ok_or(AppError::internal("Profiles not initialized"))?;
  let mut guard = store.lock().await;
  let profile = guard
    .profiles
    .iter_mut()
    .find(|p| p.id == id)
    .ok_or_else(|| AppError::not_found(format!("Profile {} not found", id)))?;
  profile.name = name;
  profile.fields = fields;
  profile.updated_at = updated_at;
  save_to_disk(&app, &guard).await
}

#[tauri::command]
#[specta::specta]
pub async fn profile_delete(app: AppHandle, id: String) -> Result<(), AppError> {
  let store = PROFILES.get().ok_or(AppError::internal("Profiles not initialized"))?;
  let mut guard = store.lock().await;
  guard.profiles.retain(|p| p.id != id);
  if guard.active_id.as_deref() == Some(id.as_str()) {
    guard.active_id = None;
  }
  save_to_disk(&app, &guard).await
}

#[tauri::command]
#[specta::specta]
pub async fn profile_set_active(app: AppHandle, id: String) -> Result<(), AppError> {
  let store = PROFILES.get().ok_or(AppError::internal("Profiles not initialized"))?;
  let mut guard = store.lock().await;
  guard.active_id = Some(id);
  save_to_disk(&app, &guard).await
}
