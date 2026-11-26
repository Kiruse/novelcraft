//! Client-side auth state for the Novelcraft social API: the signed-in
//! user, session restore at boot, and login/register/logout flows. The
//! access token lives in the pooled [`crate::api::client`]; the refresh
//! token is persisted to `{configDir}/NovelCraft/auth.json` (via the
//! engine's cfg-split storage seam) so a login survives app restarts.

use dioxus::prelude::*;
use log::warn;
use novelcraft_engine::paths::config_dir;
use novelcraft_engine::util::fs;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::{
  LoginRequest, RefreshRequest, RegisterRequest, SetUsernameRequest, TokenPair, UserResponse,
  client,
};
use crate::error::GuiError;
use crate::toast::{Toast, push_toast};

const RANDOM_USERNAME_ADJECTIVES: &[&str] = &[
  "albino",
  "amber",
  "ancient",
  "atomic",
  "bold",
  "brave",
  "bright",
  "calm",
  "clever",
  "cosmic",
  "crimson",
  "distant",
  "dreamy",
  "dusky",
  "eager",
  "fair",
  "fierce",
  "fleet",
  "frosty",
  "gentle",
  "golden",
  "grim",
  "hidden",
  "jolly",
  "kind",
  "lewd",
  "lucky",
  "lunar",
  "merry",
  "mighty",
  "mild",
  "misty",
  "noble",
  "nuclear",
  "pale",
  "primordial",
  "proud",
  "quick",
  "quiet",
  "restless",
  "rustic",
  "sage",
  "shadowy",
  "silent",
  "silver",
  "sly",
  "stellar",
  "sturdy",
  "sunny",
  "suspicious",
  "swift",
  "tidal",
  "vivid",
  "wild",
  "witty",
  "zesty",
];

const RANDOM_USERNAME_ANIMALS: &[&str] = &[
  "badger", "beaver", "bison", "bobcat", "caribou", "centaur", "cougar", "coyote", "deer", "dingo",
  "dolphin", "dragon", "eagle", "ermine", "falcon", "fox", "gecko", "griffin", "hare", "heron",
  "ibex", "ibis", "jaguar", "koala", "lemur", "lynx", "marten", "mole", "moose", "newt", "oryx",
  "otter", "owl", "pallas", "panther", "puffin", "quail", "raven", "robin", "sable", "salmon",
  "sparrow", "stoat", "sylph", "tapir", "tiger", "trout", "viper", "weasel", "wolf", "wombat",
  "wren", "yak", "zebra",
];

/// Serializes token rotations: the refresh endpoint burns the presented
/// token (single-use), so concurrent 401 handlers queue here and re-read
/// the latest token instead of re-presenting a burned one.
static REFRESH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub static AUTH: GlobalSignal<AuthState> = GlobalSignal::new(AuthState::default);

/// The signed-in account, as rendered by the title bar's `UserBox`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AuthState {
  /// The logged-in user, when signed in.
  pub user: Option<UserResponse>,
  /// True while the boot-time session restore is in flight.
  pub restoring: bool,
  /// The current refresh token — rotated on every login/refresh and
  /// persisted to disk. Consumed by logout to revoke the session.
  pub refresh_token: Option<String>,
}

/// The persisted refresh token. `None` clears the stored session.
#[derive(Debug, Serialize, Deserialize, Default)]
struct StoredToken {
  refresh_token: Option<String>,
}

fn token_path() -> Result<std::path::PathBuf, GuiError> {
  Ok(config_dir()?.join("auth.json"))
}

async fn load_stored_token() -> Option<String> {
  let path = token_path().ok()?;
  fs::deserialize::<StoredToken>(&path)
    .await
    .ok()?
    .refresh_token
}

/// Persist the refresh token (`None` clears the stored session).
async fn store_token(token: Option<String>) {
  let Ok(path) = token_path() else { return };
  if let Err(err) = fs::serialize(
    &path,
    &StoredToken {
      refresh_token: token,
    },
  )
  .await
  {
    warn!("Failed to persist auth token: {err}");
  }
}

/// Adopt a fresh token pair: hand the access token to the API client and
/// persist the (rotated) refresh token.
async fn apply_tokens(tokens: &TokenPair) {
  client().set_access_token(Some(tokens.access_token.clone()));
  AUTH.write().refresh_token = Some(tokens.refresh_token.clone());
  store_token(Some(tokens.refresh_token.clone())).await;
}

/// Drop all tokens: the client's access token, the stored refresh token.
async fn clear_tokens() {
  client().set_access_token(None);
  store_token(None).await;
}

/// Rotate the stored refresh token into a fresh pair and apply it to the
/// API client, the `AUTH` signal and the persisted session. Called by the
/// API client's `auth_refresh` wrapper when an authed request answers 401,
/// and by the GUI `ModelProvider`'s `Login` model when the LLM endpoint
/// does.
pub(crate) async fn refresh_session() -> Result<(), GuiError> {
  let _guard = REFRESH_LOCK.lock().await;
  // Re-read under the lock — a queued sibling may have rotated already.
  let Some(refresh_token) = AUTH.read().refresh_token.clone() else {
    return Err(GuiError::Api("Not signed in".to_string()));
  };
  let tokens = client().refresh(&RefreshRequest { refresh_token }).await?;
  apply_tokens(&tokens).await;
  Ok(())
}

/// Restore a persisted session at boot: rotate the stored refresh token
/// into a fresh pair and fetch the user. Silent when there is no stored
/// session or when the restore fails — the user is simply logged out.
pub async fn restore() {
  if AUTH.read().user.is_some() || AUTH.read().restoring {
    return;
  }
  let Some(token) = load_stored_token().await else {
    return;
  };
  AUTH.write().restoring = true;

  let outcome = async {
    let tokens = client()
      .refresh(&RefreshRequest {
        refresh_token: token,
      })
      .await?;
    apply_tokens(&tokens).await;
    match client().me().await {
      Ok(user) => Ok(user),
      Err(err) => {
        // Tokens without a usable user are dead weight — discard them.
        clear_tokens().await;
        Err(err)
      }
    }
  }
  .await;

  let mut state = AUTH.write();
  state.restoring = false;
  match outcome {
    Ok(user) => state.user = Some(user),
    Err(err) => warn!("Session restore failed: {err}"),
  }
}

/// Log in with email & password. On success the user is published to the
/// AUTH signal and the session is persisted.
pub async fn login(email: String, password: String) -> Result<UserResponse, GuiError> {
  let tokens = client().login(&LoginRequest { email, password }).await?;
  apply_tokens(&tokens).await;

  match client().me().await {
    Ok(user) => {
      AUTH.write().user = Some(user.clone());
      Ok(user)
    }
    Err(err) => {
      clear_tokens().await;
      Err(err)
    }
  }
}

/// Register a new account. `username` is optional — when omitted, a random
/// `adjective.animal.NNN` name is chosen; the name is set right after
/// registration via `PUT /auth/username` (the register endpoint takes no
/// username). A failed username assignment still leaves the user logged
/// in, with a toast explaining why.
pub async fn register(
  email: String,
  password: String,
  invite_code: String,
  username: Option<String>,
) -> Result<UserResponse, GuiError> {
  let tokens = client()
    .register(&RegisterRequest {
      email,
      password,
      invite_code,
    })
    .await?;
  apply_tokens(&tokens).await;

  let username = username.unwrap_or_else(random_username);
  let set_result = client()
    .set_username(&SetUsernameRequest { username })
    .await;
  if let Err(ref err) = set_result {
    error!("Failed to set username: {err}");
    push_toast(Toast::error(format!(
      "Account created, but the username could not be set: {err}"
    )));
  }

  match client().me().await {
    Ok(user) => {
      AUTH.write().user = Some(user.clone());
      Ok(user)
    }
    Err(err) => {
      clear_tokens().await;
      Err(err)
    }
  }
}

/// Log out: revoke the refresh token (best-effort — unknown tokens answer
/// 204 as well) and clear the local session & stored token.
pub async fn logout() {
  let refresh_token = AUTH.read().refresh_token.clone();
  if let Some(refresh_token) = refresh_token
    && let Err(err) = client().logout(&RefreshRequest { refresh_token }).await
  {
    warn!("Logout request failed: {err}");
  }
  clear_tokens().await;
  let mut state = AUTH.write();
  state.refresh_token = None;
  state.user = None;
}

/// Username constraints of `PUT /auth/username`: 1-30 chars of
/// [A-Za-z0-9._-], stored case-sensitively, unique case-insensitively.
pub fn valid_username(name: &str) -> bool {
  !name.is_empty()
    && name.len() <= 30
    && name
      .chars()
      .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// A random `adjective.animal.NNN` username — entropy from a fresh UUID.
pub fn random_username() -> String {
  let entropy = Uuid::new_v4().as_u128();
  let adjective = RANDOM_USERNAME_ADJECTIVES[(entropy as usize) % RANDOM_USERNAME_ADJECTIVES.len()];
  let animal = RANDOM_USERNAME_ANIMALS[((entropy >> 64) as usize) % RANDOM_USERNAME_ANIMALS.len()];
  let number = 100 + ((entropy >> 96) as usize) % 900;
  format!("{adjective}.{animal}.{number}")
}
