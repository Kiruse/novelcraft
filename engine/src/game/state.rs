use tauri::{AppHandle, Manager, State};
use tokio::sync::Mutex;

use crate::commands::llm::Models;
use crate::commands::profile::Profiles;
use crate::config::NovelCraftConfig;
use crate::error::AppError;

#[derive(Debug, Default)]
pub struct AppState {
  pub config: Mutex<NovelCraftConfig>,
  pub profiles: Mutex<Profiles>,
  pub models: Mutex<Models>,
}

impl AppState {
  pub fn get(app: &AppHandle) -> State<'_, AppState> {
    app.state::<AppState>()
  }

  pub async fn init(app: &AppHandle) -> Result<(), AppError> {
    let state = Self::get(app);

    let mut guard = state.config.lock().await;
    *guard = NovelCraftConfig::load(app).await?;

    let mut guard = state.profiles.lock().await;
    *guard = Profiles::load(app).await?;

    let mut guard = state.models.lock().await;
    *guard = Models::load(app).await?;

    Ok(())
  }
}
