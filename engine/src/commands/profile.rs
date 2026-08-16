use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::game::state::AppState;
use crate::commands::paths;
use crate::util;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
  pub id: String,
  pub name: String,
  pub fields: HashMap<String, String>,
  pub created_at: String,
  pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileListResult {
  pub profiles: Vec<Profile>,
  pub active_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profiles {
  version: u32,
  profiles: Vec<Profile>,
  active_id: Option<String>,
}

impl Profiles {
  pub async fn load() -> Result<Profiles, AppError> {
    let path = Self::default_path()?;
    if path.exists() {
      let content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
      Ok(serde_json::from_str(&content).unwrap_or_default())
    } else {
      Ok(Profiles::default())
    }
  }

  pub async fn save(&self) -> Result<(), AppError> {
    util::serialize(&Self::default_path()?, self).await
  }

  fn default_path() -> Result<PathBuf, AppError> {
    Ok(paths::data_dir()?.join("profiles.json"))
  }
}

impl Default for Profiles {
  fn default() -> Self {
    Self {
      version: 1,
      profiles: vec![],
      active_id: None,
    }
  }
}

pub async fn profile_list(state: &AppState) -> Result<ProfileListResult, AppError> {
  let guard = state.profiles.lock().await;
  Ok(ProfileListResult {
    profiles: guard.profiles.clone(),
    active_id: guard.active_id.clone(),
  })
}

pub async fn profile_create(
  state: &AppState,
  id: String,
  name: String,
  fields: HashMap<String, String>,
  created_at: String,
) -> Result<(), AppError> {
  let mut guard = state.profiles.lock().await;
  guard.profiles.push(Profile {
    id,
    name,
    fields,
    created_at: created_at.clone(),
    updated_at: created_at,
  });
  guard.save().await
}

pub async fn profile_update(
  state: &AppState,
  id: String,
  name: String,
  fields: HashMap<String, String>,
  updated_at: String,
) -> Result<(), AppError> {
  let mut guard = state.profiles.lock().await;
  let profile = guard
    .profiles
    .iter_mut()
    .find(|p| p.id == id)
    .ok_or_else(|| AppError::not_found(format!("Profile {} not found", id)))?;
  profile.name = name;
  profile.fields = fields;
  profile.updated_at = updated_at;
  guard.save().await
}

pub async fn profile_delete(state: &AppState, id: String) -> Result<(), AppError> {
  let mut guard = state.profiles.lock().await;
  guard.profiles.retain(|p| p.id != id);
  if guard.active_id.as_deref() == Some(id.as_str()) {
    guard.active_id = None;
  }
  guard.save().await
}

pub async fn profile_set_active(state: &AppState, id: String) -> Result<(), AppError> {
  let mut guard = state.profiles.lock().await;
  guard.active_id = Some(id);
  guard.save().await
}
