//! Typed client for the Novelcraft social API, mirroring the OpenAPI spec
//! in `docs/api.json`. The `/chat/*` endpoints are intentionally absent —
//! the inference provider is the user's choice (engine model config), so
//! the app never calls them.

// The full spec surface lives here ahead of its GUI wiring — the
// not-yet-called endpoints/methods are intentional.
#![allow(dead_code)]

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock, RwLock};

use bytes::Bytes;
use futures_util::stream::{Stream, StreamExt};
use sse_core::{MessageEvent, SseEvent, SseStream, SseStreamError};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::error::GuiError;

// TODO: The base URL is hardcoded until the API gets a proper domain.

/// Base URL of the Novelcraft API.
pub const API_BASE_URL: &str = "https://api.novelcraft.quest";

/// The process-wide API client (connection pooling + current access token).
pub fn client() -> &'static ApiClient {
  static CLIENT: OnceLock<ApiClient> = OnceLock::new();
  CLIENT.get_or_init(|| ApiClient::new(API_BASE_URL))
}

// ---- Schemas (docs/api.json → components/schemas) ----

/// An inspiration for a new story, as served by `GET /inspirations`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Inspiration {
  pub title: String,
  /// Story premise (markdown, rendered with the shared `Markdown` component).
  pub premise: String,
  /// Optional author-authored style & behavior guidance.
  #[serde(default)]
  pub creative_direction: Option<String>,
  /// Tags this inspiration is filed under.
  pub tags: Vec<String>,
}

/// Initial token pair from `POST /auth/register` / `/auth/login`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TokenPair {
  pub access_token: String,
  pub refresh_token: String,
}

/// The current user (`GET /auth/me`, `PUT /auth/username`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct UserResponse {
  pub id: String,
  pub email: String,
  /// Balance in quills, rounded to 2 decimals.
  pub quills: f64,
  pub username: Option<String>,
}

/// The quill balance (`GET /quills`, `POST /quills/claim`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct BalanceResponse {
  /// Current balance, rounded to 2 decimals.
  pub quills: f64,
}

/// One ledger entry of the quill earn/spend history.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct QuillHistoryResponse {
  pub id: i64,
  pub user_id: String,
  pub amount: f64,
  pub kind: String,
  /// Groups multi-round chat requests into one accumulating record.
  pub tag: Option<String>,
  /// RFC3339 timestamp.
  pub created_at: String,
}

/// A page of the quill ledger — stateless keyset pagination: `has_more`
/// marks older entries, anchored at the last id in `result`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HistoryResponse {
  pub result: Vec<QuillHistoryResponse>,
  pub has_more: bool,
}

/// Metadata of one occupied save slot — everything a sync client needs to
/// decide which side is newer; the blobs themselves are opaque.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SaveMeta {
  /// The slot number, 1-3.
  pub slot: u8,
  /// Blob size in bytes (at most 50 MiB).
  pub size_bytes: u64,
  /// When the server last wrote this slot (RFC3339).
  pub updated_at: String,
}

/// The occupied save slots (`GET /saves`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SavesResponse {
  pub saves: Vec<SaveMeta>,
}

/// One entry of the supported-models list (`GET /models`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ModelEntry {
  /// The model identifier handed to the completions endpoint.
  pub id: String,
}

/// The supported inference models (`GET /models`) — the OpenAI-shaped
/// `/chat/models` list behind a friendlier alias.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ModelsResponse {
  pub data: Vec<ModelEntry>,
}

impl ModelsResponse {
  /// The model identifiers, sorted alphabetically.
  pub fn ids(&self) -> Vec<String> {
    let mut ids: Vec<String> = self.data.iter().map(|model| model.id.clone()).collect();
    ids.sort();
    ids
  }
}

/// `POST /auth/forgot-password` — always answers 202.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Accepted {
  pub status: String,
}

/// One event of the account event stream (`GET /events`).
///
/// Today the only event is [`AccountEvent::Quills`] — the balance changed.
/// Unrecognized event kinds surface as [`AccountEvent::Unknown`] so a
/// server-side extension can't wedge the stream.
#[derive(Debug, Clone, PartialEq)]
pub enum AccountEvent {
  /// The quill balance changed; the payload is the new balance, rounded
  /// to 2 decimals like every other balance response.
  Quills { quills: f64 },
  /// An unrecognized event kind, preserved as-is.
  Unknown { event: String, data: String },
}

impl AccountEvent {
  /// Parses a raw SSE message: the `quills` kind carries the bare balance
  /// (or, leniently, the full balance response); anything else stays raw.
  fn parse(message: MessageEvent) -> Self {
    if message.event == "quills" {
      let data = message.data.trim();
      if let Ok(quills) = data.parse::<f64>() {
        return Self::Quills { quills };
      }
      if let Ok(balance) = serde_json::from_str::<BalanceResponse>(data) {
        return Self::Quills { quills: balance.quills };
      }
    }
    Self::Unknown {
      event: message.event.into_owned(),
      data: message.data,
    }
  }
}

#[derive(Debug, Serialize)]
pub struct RegisterRequest {
  pub email: String,
  pub password: String,
  pub invite_code: String,
}

#[derive(Debug, Serialize)]
pub struct LoginRequest {
  pub email: String,
  pub password: String,
}

#[derive(Debug, Serialize)]
pub struct RefreshRequest {
  pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct SetUsernameRequest {
  pub username: String,
}

#[derive(Debug, Serialize)]
pub struct ForgotPasswordRequest {
  pub email: String,
}

#[derive(Debug, Serialize)]
pub struct ResetPasswordRequest {
  pub token: String,
  pub new_password: String,
}

/// The uniform error shape of every non-2xx response.
#[derive(Deserialize)]
struct ErrorBody {
  error: String,
}

// ---- Client ----

/// Typed client for the Novelcraft social API. Cheap to clone (the HTTP
/// client is pooled); clones share the access token.
#[derive(Clone)]
pub struct ApiClient {
  http: reqwest::Client,
  base_url: String,
  access_token: Arc<RwLock<Option<String>>>,
}

impl ApiClient {
  /// Creates a client for `base_url` without an access token.
  pub fn new(base_url: &str) -> Self {
    Self {
      http: reqwest::Client::new(),
      base_url: base_url.to_string(),
      access_token: Arc::new(RwLock::new(None)),
    }
  }

  /// Sets or clears the bearer token used by authenticated endpoints —
  /// `Some(access_token)` after login/register/refresh, `None` on logout.
  pub fn set_access_token(&self, token: Option<String>) {
    *self.access_token.write().unwrap() = token;
  }

  /// The current access token, if any.
  pub fn access_token(&self) -> Option<String> {
    self.access_token.read().unwrap().clone()
  }

  fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
    self
      .http
      .request(method, format!("{}{path}", self.base_url))
  }

  /// `request` + bearer header; errors before sending when no token is set.
  fn authed(
    &self,
    method: reqwest::Method,
    path: &str,
  ) -> Result<reqwest::RequestBuilder, GuiError> {
    let token = self.access_token.read().unwrap().clone();
    token
      .map(|token| self.request(method, path).bearer_auth(token))
      .ok_or_else(|| GuiError::Api("Not signed in".to_string()))
  }

  // ---- inspirations (public) ----

  /// `GET /inspirations` — every inspiration sorted by title; with `tags`,
  /// only those tagged with ALL of the given tags (an empty selection
  /// matches everything).
  pub async fn list_inspirations(&self, tags: &[String]) -> Result<Vec<Inspiration>, GuiError> {
    let mut request = self.request(reqwest::Method::GET, "/inspirations");
    if !tags.is_empty() {
      request = request.query(&[("tags", tags.join(","))]);
    }
    send(request).await
  }

  /// `GET /inspirations/tags` — every tag in use (flat, sorted, deduped).
  pub async fn list_inspiration_tags(&self) -> Result<Vec<String>, GuiError> {
    send(self.request(reqwest::Method::GET, "/inspirations/tags")).await
  }

  // ---- models (public) ----

  /// `GET /models` — the supported inference models: the OpenAI-shaped
  /// `/chat/models` list behind a friendlier alias, public so anonymous
  /// clients (e.g. onboarding, signed-out settings) can render model
  /// pickers.
  pub async fn list_models(&self) -> Result<ModelsResponse, GuiError> {
    send(self.request(reqwest::Method::GET, "/models")).await
  }

  // ---- auth ----

  /// `POST /auth/register` — creates the account, returns the initial
  /// token pair. The invite code is matched case-insensitively.
  pub async fn register(&self, body: &RegisterRequest) -> Result<TokenPair, GuiError> {
    send(
      self
        .request(reqwest::Method::POST, "/auth/register")
        .json(body),
    )
    .await
  }

  /// `POST /auth/login`.
  pub async fn login(&self, body: &LoginRequest) -> Result<TokenPair, GuiError> {
    send(
      self
        .request(reqwest::Method::POST, "/auth/login")
        .json(body),
    )
    .await
  }

  /// `POST /auth/refresh` — rotates a refresh token into a fresh pair;
  /// the presented token is burned (single-use).
  pub async fn refresh(&self, body: &RefreshRequest) -> Result<TokenPair, GuiError> {
    send(
      self
        .request(reqwest::Method::POST, "/auth/refresh")
        .json(body),
    )
    .await
  }

  /// `POST /auth/logout` — revokes a refresh token; best-effort, unknown
  /// tokens answer 204 as well.
  pub async fn logout(&self, body: &RefreshRequest) -> Result<(), GuiError> {
    send_empty(
      self
        .request(reqwest::Method::POST, "/auth/logout")
        .json(body),
    )
    .await
  }

  /// `GET /auth/me`.
  pub async fn me(&self) -> Result<UserResponse, GuiError> {
    auth_refresh(|| async move {
      send(self.authed(reqwest::Method::GET, "/auth/me")?).await
    })
    .await
  }

  /// `PUT /auth/username` — 1-30 chars of [A-Za-z0-9._-], stored
  /// case-sensitive, unique case-insensitively.
  pub async fn set_username(&self, body: &SetUsernameRequest) -> Result<UserResponse, GuiError> {
    auth_refresh(|| async move {
      send(
        self
          .authed(reqwest::Method::PUT, "/auth/username")?
          .json(body),
      )
      .await
    })
    .await
  }

  /// `DELETE /auth/username` — clears the username, releasing it.
  pub async fn clear_username(&self) -> Result<(), GuiError> {
    auth_refresh(|| async move {
      send_empty(self.authed(reqwest::Method::DELETE, "/auth/username")?).await
    })
    .await
  }

  /// `POST /auth/forgot-password` — always answers 202, never reveals
  /// whether the account exists.
  pub async fn forgot_password(&self, body: &ForgotPasswordRequest) -> Result<Accepted, GuiError> {
    send(
      self
        .request(reqwest::Method::POST, "/auth/forgot-password")
        .json(body),
    )
    .await
  }

  /// `POST /auth/reset-password` — single-use token; revokes all of the
  /// user's sessions.
  pub async fn reset_password(&self, body: &ResetPasswordRequest) -> Result<(), GuiError> {
    send_empty(
      self
        .request(reqwest::Method::POST, "/auth/reset-password")
        .json(body),
    )
    .await
  }

  // ---- quills ----

  /// `GET /quills` — the current quill balance.
  pub async fn quills_balance(&self) -> Result<BalanceResponse, GuiError> {
    auth_refresh(|| async move {
      send(self.authed(reqwest::Method::GET, "/quills")?).await
    })
    .await
  }

  /// `POST /quills/claim` — grants 10.00 quills once per UTC calendar day.
  pub async fn claim_quills(&self) -> Result<BalanceResponse, GuiError> {
    auth_refresh(|| async move {
      send(self.authed(reqwest::Method::POST, "/quills/claim")?).await
    })
    .await
  }

  /// `GET /quills/history` — the earn/spend ledger, newest first. Keyset
  /// pagination: the next page anchors at the last id in `result` while
  /// `has_more` is true, so pages never shift when new entries arrive.
  pub async fn quills_history(
    &self,
    from_id: Option<i64>,
    limit: Option<i64>,
  ) -> Result<HistoryResponse, GuiError> {
    auth_refresh(|| async move {
      let mut request = self.authed(reqwest::Method::GET, "/quills/history")?;
      if let Some(from_id) = from_id {
        request = request.query(&[("from_id", from_id)]);
      }
      if let Some(limit) = limit {
        request = request.query(&[("limit", limit)]);
      }
      send(request).await
    })
    .await
  }

  // ---- saves ----

  /// `GET /saves` — the occupied save slots, ascending by slot number.
  pub async fn list_saves(&self) -> Result<SavesResponse, GuiError> {
    auth_refresh(|| async move {
      send(self.authed(reqwest::Method::GET, "/saves")?).await
    })
    .await
  }

  /// `POST /saves` — stores the (opaque, end-to-end encrypted) blob in the
  /// lowest free slot; rejected with 409 when all three are occupied —
  /// overwrite a slot explicitly with [`ApiClient::write_save`] instead.
  pub async fn push_save(&self, blob: Vec<u8>) -> Result<SaveMeta, GuiError> {
    auth_refresh(move || {
      let blob = blob.clone();
      async move {
        send(octet(self.authed(reqwest::Method::POST, "/saves")?, blob)).await
      }
    })
    .await
  }

  /// `GET /saves/{slot}` — the raw end-to-end-encrypted blob.
  pub async fn save_blob(&self, slot: u8) -> Result<Vec<u8>, GuiError> {
    auth_refresh(|| async move {
      let response = self
        .authed(reqwest::Method::GET, &format!("/saves/{slot}"))?
        .send()
        .await
        .map_err(|e| GuiError::Api(e.to_string()))?;
      if !response.status().is_success() {
        return Err(api_error(response).await);
      }
      Ok(
        response
          .bytes()
          .await
          .map_err(|e| GuiError::Api(e.to_string()))?
          .to_vec(),
      )
    })
    .await
  }

  /// `PUT /saves/{slot}` — creates or overwrites the slot (last write
  /// wins). Slots are 1-3.
  pub async fn write_save(&self, slot: u8, blob: Vec<u8>) -> Result<SaveMeta, GuiError> {
    auth_refresh(move || {
      let blob = blob.clone();
      async move {
        send(octet(
          self.authed(reqwest::Method::PUT, &format!("/saves/{slot}"))?,
          blob,
        ))
        .await
      }
    })
    .await
  }

  /// `DELETE /saves/{slot}` — releases the slot; idempotent, an empty slot
  /// answers 204 as well.
  pub async fn delete_save(&self, slot: u8) -> Result<(), GuiError> {
    auth_refresh(|| async move {
      send_empty(self.authed(reqwest::Method::DELETE, &format!("/saves/{slot}"))?).await
    })
    .await
  }

  // ---- events ----

  /// `GET /events` — a server-sent event stream of the account's events.
  /// Today the only event is [`AccountEvent::Quills`] — the balance
  /// changed — carrying the new balance as the data payload. Delivery is
  /// best-effort: reconnecting clients should re-fetch the balance, and
  /// since every event carries full state, a missed event self-heals on
  /// the next one.
  pub async fn events_stream(&self) -> Result<EventStream, GuiError> {
    auth_refresh(|| async move {
      let response = self
        .authed(reqwest::Method::GET, "/events")?
        .header(reqwest::header::ACCEPT, "text/event-stream")
        .send()
        .await
        .map_err(|e| GuiError::Api(e.to_string()))?;
      if !response.status().is_success() {
        return Err(api_error(response).await);
      }
      Ok(EventStream::new(response))
    })
    .await
  }
}

/// Sends the request, mapping non-2xx responses to the API's uniform
/// error shape, and decodes the JSON body.
async fn send<T: DeserializeOwned>(request: reqwest::RequestBuilder) -> Result<T, GuiError> {
  let response = request
    .send()
    .await
    .map_err(|e| GuiError::Api(e.to_string()))?;
  if !response.status().is_success() {
    return Err(api_error(response).await);
  }
  response
    .json::<T>()
    .await
    .map_err(|e| GuiError::Api(e.to_string()))
}

/// `GET {base_url}/models` — the model list of a user-configured
/// OpenAI-compatible endpoint, the model-picker counterpart of the custom
/// inference config. `api_key` is attached as the bearer when given.
pub async fn list_models_at(
  base_url: &str,
  api_key: Option<&str>,
) -> Result<ModelsResponse, GuiError> {
  let mut request = reqwest::Client::new()
    .get(format!("{}/models", base_url.trim_end_matches('/')));
  if let Some(api_key) = api_key {
    request = request.bearer_auth(api_key);
  }
  send(request).await
}

/// Like [`send`] for endpoints without a response body (204).
async fn send_empty(request: reqwest::RequestBuilder) -> Result<(), GuiError> {
  let response = request
    .send()
    .await
    .map_err(|e| GuiError::Api(e.to_string()))?;
  if !response.status().is_success() {
    return Err(api_error(response).await);
  }
  Ok(())
}

/// Runs an authed request with a single automatic recovery: a 401 means
/// the short-lived access token expired — rotate the token pair once
/// ([`crate::auth::refresh_session`]) and re-invoke `request`, which must
/// rebuild the request so the retried call carries the fresh token. Only a
/// second failure surfaces. Signed-out calls (`authed` errs before
/// sending) fail fast without touching the refresh flow.
pub(crate) async fn auth_refresh<T, F, Fut>(mut request: F) -> Result<T, GuiError>
where
  F: FnMut() -> Fut,
  Fut: Future<Output = Result<T, GuiError>>,
{
  match request().await {
    Err(GuiError::ApiStatus { status: 401, .. }) => {}
    other => return other,
  }
  crate::auth::refresh_session().await?;
  request().await
}

/// Builds the error from a non-2xx response: the server message from the
/// uniform error body, falling back to the bare status line.
async fn api_error(response: reqwest::Response) -> GuiError {
  let status = response.status();
  let message = response
    .json::<ErrorBody>()
    .await
    .map(|body| body.error)
    .unwrap_or_else(|_| status.to_string());
  GuiError::ApiStatus {
    status: status.as_u16(),
    message,
  }
}

/// Attaches an octet-stream body (the save blobs' content type).
fn octet(request: reqwest::RequestBuilder, blob: Vec<u8>) -> reqwest::RequestBuilder {
  request
    .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
    .body(blob)
}

// ---- SSE stream plumbing ----

/// A byte stream of the SSE response. Deliberately without `Send` bounds —
/// the browser fetch streams backing the web target aren't.
type ByteStream = Pin<Box<dyn Stream<Item = reqwest::Result<Bytes>>>>;

/// The streamed `GET /events` response — a server-sent event stream of
/// the account's events, parsed by [`SseStream`] (the same `sse-core`
/// parser the engine's agent loop uses).
pub struct EventStream {
  stream: SseStream<ByteStream>,
}

impl EventStream {
  fn new(response: reqwest::Response) -> Self {
    Self {
      stream: SseStream::new(Box::pin(response.bytes_stream())),
    }
  }

  /// The next event of the stream, or `None` once the server closes it.
  ///
  /// Under normal operation the stream never ends — treat a `None` as a
  /// disconnect and reconnect (re-fetching the balance first, per the
  /// endpoint's best-effort delivery).
  pub async fn next_event(&mut self) -> Result<Option<AccountEvent>, GuiError> {
    loop {
      let Some(event) = self.stream.next().await else {
        return Ok(None);
      };
      match event {
        Ok(SseEvent::Message(message)) => return Ok(Some(AccountEvent::parse(message))),
        Ok(SseEvent::Retry(_)) => continue, // server-advised retry delay; nothing to do
        Err(SseStreamError::Inner(err)) => return Err(GuiError::Api(err.to_string())),
        Err(SseStreamError::PayloadTooLarge(_)) => {
          // The decoder dropped the oversized event and resynchronized at
          // the next boundary; the connection stays open and every event
          // carries full state, so the loss self-heals on the next one.
          continue;
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Builds a raw SSE message event.
  fn message(event: &str, data: &str) -> MessageEvent {
    MessageEvent {
      event: event.to_owned().into(),
      data: data.to_owned(),
      last_event_id: None,
    }
  }

  #[test]
  fn parses_a_quill_event() {
    assert_eq!(
      AccountEvent::parse(message("quills", "10.50")),
      AccountEvent::Quills { quills: 10.5 }
    );
  }

  #[test]
  fn parses_a_quill_event_with_a_json_payload() {
    assert_eq!(
      AccountEvent::parse(message("quills", r#"{"quills": 10.50}"#)),
      AccountEvent::Quills { quills: 10.5 }
    );
  }

  #[test]
  fn keeps_unrecognized_event_kinds_raw() {
    assert_eq!(
      AccountEvent::parse(message("future", "…")),
      AccountEvent::Unknown {
        event: "future".to_owned(),
        data: "…".to_owned(),
      }
    );
  }

  #[test]
  fn keeps_unparseable_quill_payloads_raw() {
    assert_eq!(
      AccountEvent::parse(message("quills", "garbage")),
      AccountEvent::Unknown {
        event: "quills".to_owned(),
        data: "garbage".to_owned(),
      }
    );
  }
}
