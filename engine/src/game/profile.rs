use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileV1 {
  pub id: String,
  pub name: String,
  pub fields: HashMap<String, String>,
  pub created_at: String,
  pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profiles {
  pub version: u32,
  pub profiles: Vec<ProfileV1>,
  pub active_id: Option<String>,
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
    crate::util::serialize(&Self::default_path()?, self).await
  }

  fn default_path() -> Result<PathBuf, AppError> {
    Ok(crate::paths::data_dir()?.join("profiles.json"))
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
