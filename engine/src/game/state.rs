use std::sync::Arc;

use tokio::sync::Mutex;

use crate::commands::llm::Models;
use crate::commands::profile::Profiles;
use crate::config::NovelCraftConfig;
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct AppState {
  pub config: Arc<Mutex<NovelCraftConfig>>,
  pub profiles: Arc<Mutex<Profiles>>,
  pub models: Arc<Mutex<Models>>,
}

impl AppState {
  pub async fn init() -> Result<AppState, AppError> {
    let config = NovelCraftConfig::load().await?;
    let profiles = Profiles::load().await?;
    let models = Models::load().await?;

    Ok(Self {
      config: Arc::new(Mutex::new(config)),
      profiles: Arc::new(Mutex::new(profiles)),
      models: Arc::new(Mutex::new(models)),
    })
  }
}
