use std::ffi::OsString;

use kiruklaw_agent_loop::Conversation;

use crate::error::AppError;
use crate::game::pages::PageBatch;
use crate::game::profile::ProfileV1;
use crate::game::session::SessionV1;

#[derive(Debug, Clone, Default)]
pub struct GameEngine {
  /// Active user session
  pub session: Option<SessionV1>,
  /// Active user profile
  pub profile: Option<ProfileV1>,
}

impl GameEngine {
  pub fn with_session(self, session: SessionV1) -> Self {
    Self {
      session: Some(session),
      ..self
    }
  }
  pub fn with_profile(self, profile: ProfileV1) -> Self {
    Self {
      profile: Some(profile),
      ..self
    }
  }

  fn session(&self) -> Result<&SessionV1, AppError> {
    self.session.as_ref().ok_or(AppError::state("no active session"))
  }
  fn session_mut(&mut self) -> Result<&mut SessionV1, AppError> {
    self.session.as_mut().ok_or(AppError::state("no active session"))
  }
  fn session_id(&self) -> Result<&String, AppError> {
    Ok(&self.session()?.id)
  }

  pub async fn list_sessions() -> Result<Vec<OsString>, AppError> {
    let path = SessionV1::root()?;
    let mut dir_iter = tokio::fs::read_dir(&path).await?;
    let mut result = Vec::new();
    while let Some(entry) = dir_iter.next_entry().await? {
      result.push(entry.file_name());
    }
    Ok(result)
  }

  pub fn history(&self) -> Result<&Conversation, AppError> {
    Ok(&self.session()?.conversation)
  }

  /// Push a new user prompt to the end of the current session.
  pub async fn prompt(&mut self, content: String) -> Result<(), AppError> {
    todo!()
  }

  /// Rewrite the last page under consideration of the given instructions.
  pub async fn rewrite(&mut self, instruct: String) -> Result<(), AppError> {
    todo!()
  }

  /// Fork this session from the given page index, deleting all subsequent pages.
  pub async fn fork(&mut self, page_index: usize) -> Result<(), AppError> {
    let batch_index = PageBatch::batch_of(page_index);
    Self::truncate_batches(self.session_id()?, batch_index).await?;
    todo!()
  }

  /// Delete all batches after the given `batch_index`, with this `batch_index` becoming
  /// the last retained batch.
  async fn truncate_batches(sid: &String, batch_index: usize) -> Result<(), AppError> {
    let batches = SessionV1::batches(sid).await?;
    let batches = batches
      .iter()
      .map(|b| PageBatch::parse_batch_idx(b.to_string_lossy()))
      .filter(Option::is_some)
      .map(|idx| idx.unwrap())
      .filter(|idx| *idx > batch_index);
    let dir = SessionV1::dir(sid)?;
    for batch in batches {
      let path = PageBatch::join_path(&dir, batch);
      tokio::fs::remove_file(&path).await?;
    }
    Ok(())
  }
}
