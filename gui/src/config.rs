use std::borrow::Cow;
use std::ops::{Deref, DerefMut};

use novelcraft_engine::agent_loop::{AgentLoopError, OpenAiCompatible};
use novelcraft_engine::config::{ConfigProvider, DEFAULT_HOST, ModelConfig, ModelPurpose, NovelCraftConfig};
use novelcraft_engine::game::engine::ModelProvider;
use novelcraft_engine::util::fs;
use serde::{Deserialize, Serialize};

use crate::error::GuiError;

/// The engine config, plus GUI-owned fields. The engine is generic over a
/// [`ConfigProvider`] + [`ModelProvider`]; this GUI wrapper implements both,
/// resolving [`ModelConfig::Login`] variants against the signed-in
/// NovelCraft account (see [`Model`]).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GuiConfig {
  #[serde(flatten)]
  pub base: NovelCraftConfig,
  /// Whether the first-boot onboarding was completed. GUI-owned (the
  /// engine's config knows nothing about it): fresh installs start at
  /// `false` via [`GuiConfig::default`]; config files predating the
  /// GUI-owned flag default to `true` so existing users are not
  /// re-onboarded.
  #[serde(default = "GuiConfig::default_onboarded")]
  pub onboarded: bool,
}

impl GuiConfig {
  fn default_onboarded() -> bool {
    true
  }

  /// Persist the whole config (engine fields + the GUI-owned onboarding
  /// flag) to the engine's config path.
  pub async fn save(&self) -> Result<(), GuiError> {
    fs::serialize(&NovelCraftConfig::default_path()?, self).await?;
    Ok(())
  }

  pub async fn load() -> Result<GuiConfig, GuiError> {
    let base = NovelCraftConfig::load_json().await?;
    serde_json::from_value(base)
      .map_err(|e| GuiError::Config(format!("{e}")))
  }
}

impl ConfigProvider for GuiConfig {
  fn get_config(&self) -> &NovelCraftConfig {
    &self.base
  }
  fn get_config_mut(&mut self) -> &mut NovelCraftConfig {
    &mut self.base
  }
}

impl ModelProvider for GuiConfig {
  fn get_model(&self, purpose: ModelPurpose) -> impl OpenAiCompatible {
    Model(self.base.models[purpose].clone())
  }
}

impl Deref for GuiConfig {
  type Target = NovelCraftConfig;

  fn deref(&self) -> &Self::Target {
    &self.base
  }
}

impl DerefMut for GuiConfig {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.base
  }
}

/// A [`ModelConfig`] resolved into a requestable model. `Login` variants
/// assemble their data from across the app: the hosted inference host is
/// the shared default host and the bearer is the signed-in account's
/// access token, rotated once on a mid-run 401.
struct Model(ModelConfig);

impl OpenAiCompatible for Model {
  fn base_url(&self) -> Cow<'_, str> {
    match &self.0 {
      ModelConfig::Login { .. } => Cow::Borrowed(DEFAULT_HOST),
      ModelConfig::OpenAiCompatible { base_url, .. } => Cow::Borrowed(base_url),
    }
  }

  fn model(&self) -> Cow<'_, str> {
    match &self.0 {
      ModelConfig::Login { model } |
      ModelConfig::OpenAiCompatible { model, .. }=> Cow::Borrowed(model),
    }
  }

  fn auth(&self) -> Option<Cow<'_, str>> {
    match &self.0 {
      ModelConfig::Login { .. } =>
        crate::api::client()
          .access_token()
          .map(Cow::Owned),
      ModelConfig::OpenAiCompatible { api_key, .. } =>
        Some(api_key.as_str())
          .filter(|key| !key.trim().is_empty())
          .map(Cow::Borrowed),
    }
  }

  async fn refresh_auth(&self) -> Result<Option<Cow<'_, str>>, AgentLoopError> {
    crate::auth::refresh_session()
      .await
      .map_err(|_| AgentLoopError::Unauthorized)?;
    Ok(self.auth())
  }
}
