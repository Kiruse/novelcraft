use std::cmp::max;
use std::ffi::OsString;

use tauri::AppHandle;

use crate::error::AppError;
use crate::game::pages::{PageBatch, PageV1, ResponseV1};
use crate::game::session::SessionV1;

pub struct GameEngine<'a> {
  app: &'a AppHandle,
  /// Active game session, if any
  session: Option<GameSession>,
}

/// Active session abstraction
struct GameSession {
  /// Active story session
  session: SessionV1,
  /// Last 2 page batches of this session
  page_batches: (PageBatch, PageBatch),
}

impl GameSession {
  fn batch<'a>(&'a self, batch_index: usize) -> Option<&'a PageBatch> {
    if batch_index == self.page_batches.0.offset {
      Some(&self.page_batches.0)
    } else if batch_index == self.page_batches.1.offset {
      Some(&self.page_batches.1)
    } else {
      None
    }
  }
}

#[derive(Debug, Clone, Default)]
pub enum UpdateOp<T> {
  #[default]
  Ignore,
  Update(T),
}

impl<T> UpdateOp<T> {
  pub fn apply(self, value: T) -> T {
    match self {
      UpdateOp::Ignore => value,
      UpdateOp::Update(v) => v,
    }
  }
}

#[derive(Debug, Clone, Default)]
pub struct UpdatePageOps {
  pub system: UpdateOp<Option<String>>,
  pub prompt: UpdateOp<Option<String>>,
  pub responses: UpdateOp<Vec<ResponseV1>>,
}

impl UpdatePageOps {
  pub fn apply(self, page: PageV1) -> PageV1 {
    PageV1 {
      prompt: self.prompt.apply(page.prompt),
      system: self.system.apply(page.system),
      responses: self.responses.apply(page.responses),
    }
  }
}

impl<'a> GameEngine<'a> {
  pub fn from_app(app: &'a AppHandle) -> GameEngine<'a> {
    Self {
      app,
      session: None,
    }
  }
  pub async fn from_app_and_session_id(app: &'a AppHandle, session_id: &String) -> Result<GameEngine<'a>, AppError> {
    let (session, batches) = tokio::join!(
      SessionV1::load(app, &session_id),
      Self::load_tail_batches(app, session_id.clone()),
    );
    Ok(Self {
      app,
      session: Some(GameSession {
        session: session?,
        page_batches: batches?,
      }),
    })
  }

  fn session(&self) -> Result<&GameSession, AppError> {
    self.session.as_ref().ok_or(AppError::state("no active session"))
  }
  fn session_mut(&mut self) -> Result<&mut GameSession, AppError> {
    self.session.as_mut().ok_or(AppError::state("no active session"))
  }
  fn session_id(&self) -> Result<&String, AppError> {
    Ok(&self.session()?.session.id)
  }

  /// List the saved sessions by ID.
  pub async fn list_sessions(app: &AppHandle) -> Result<Vec<OsString>, AppError> {
    let path = SessionV1::root(app)?;
    let mut dir_iter = tokio::fs::read_dir(&path).await?;
    let mut result = Vec::new();
    while let Some(entry) = dir_iter.next_entry().await? {
      result.push(entry.file_name());
    }
    Ok(result)
  }

  /// Get the most recent `count` pages for prompting.
  pub fn history<'b>(&'b self, count: usize) -> Result<Vec<&'b PageV1>, AppError> {
    let session = self.session()?;
    let total_pages = session.page_batches.0.pages.len() + session.page_batches.1.pages.len();

    let count = max(count, total_pages);
    Ok(session.page_batches.0.pages.iter()
      .chain(session.page_batches.1.pages.iter())
      .skip(total_pages - count)
      .collect::<Vec<_>>())
  }

  pub async fn page(&self, page_index: usize) -> Result<PageV1, AppError> {
    let session = self.session()?;
    let page_batch = PageBatch::batch_of(page_index);
    if let Some(loaded_batch) = session.batch(page_batch) {
      Ok(loaded_batch.pages[PageBatch::page_offset(page_index)].clone())
    } else {
      let batch = PageBatch::load(self.app, session.session.id.clone(), page_batch).await?;
      Ok(batch.pages[PageBatch::page_offset(page_index)].clone())
    }
  }

  /// Create a new page with the given prompt.
  pub async fn create_page(&mut self, prompt: String) -> Result<(), AppError> {
    let app = self.app.clone();
    let session = self.session_mut()?;
    session.session.page_count += 1;

    if session.page_batches.1.is_full() {
      session.session.batch_count += 1;
      std::mem::swap(&mut session.page_batches.0, &mut session.page_batches.1);
      session.page_batches.1 = PageBatch {
        offset: session.page_batches.1.offset + 1,
        session_id: session.session.id.clone(),
        ..Default::default()
      };
    }

    session.page_batches.1.pages.push(PageV1 {
      prompt: Some(prompt),
      ..Default::default()
    });
    session.page_batches.1.save(&app).await?;
    Ok(())
  }

  /// Update the last page in the active session.
  pub async fn update_page(&mut self, ops: UpdatePageOps) -> Result<(), AppError> {
    let session = self.session_mut()?;
    if session.session.page_count == 0 {
      return Err(AppError::state("empty session"));
    }

    // This little trick works only cus we exclusively work on the last page
    let page = session.page_batches.1.pages.pop().unwrap();
    session.page_batches.1.pages.push(ops.apply(page));
    Ok(())
  }

  /// Fork the current page history, truncating all pages after `page_index`.
  ///
  /// **Note:** In the future, this may/will be changed to allow "parallel timelines"
  /// rather than annihilating this one session.
  pub async fn fork(&mut self, page_index: usize) -> Result<(), AppError> {
    let batch_index = PageBatch::batch_of(page_index);
    Self::truncate_batches(self.app, self.session_id()?, batch_index).await?;

    if batch_index != self.session()?.page_batches.1.offset {
      self.session_mut()?.page_batches = Self::load_tail_batches(self.app, self.session_id()?.clone()).await?;
    }

    let cutoff = page_index - self.session()?.page_batches.1.offset * PageBatch::MAX_PAGES_PER_BATCH;
    self.session_mut()?.page_batches.1.pages.truncate(cutoff);
    Ok(())
  }

  /// Truncate page batches after the given `batch_index`
  async fn truncate_batches(app: &AppHandle, sid: &String, batch_index: usize) -> Result<(), AppError> {
    let batches = SessionV1::batches(app, sid).await?;
    let batches = batches
      .iter()
      .map(|b| PageBatch::parse_batch_idx(b.to_string_lossy()))
      .filter(Option::is_some)
      .map(|idx| idx.unwrap())
      .filter(|idx| *idx > batch_index);
    let dir = SessionV1::dir(app, sid)?;
    for batch in batches {
      let path = PageBatch::join_path(&dir, batch);
      tokio::fs::remove_file(&path).await?;
    }
    Ok(())
  }

  /// Load the last 2 `PageBatch`es for the given session
  async fn load_tail_batches(app: &AppHandle, sid: String) -> Result<(PageBatch, PageBatch), AppError> {
    let batch_idx = SessionV1::count_batches(app, &sid).await?;
    Ok((PageBatch::load(app, sid.clone(), batch_idx - 1).await?, PageBatch::load(app, sid, batch_idx).await?))
  }
}
