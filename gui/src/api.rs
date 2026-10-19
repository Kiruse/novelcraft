use std::sync::OnceLock;

use serde::de::DeserializeOwned;

use crate::error::GuiError;

// TODO: The base URL is hardcoded until the inspiration service gets a
// proper domain.

/// Base URL of the inspiration service.
pub const API_BASE_URL: &str = "https://novelcraft-api.onrender.com";

/// GETs `{API_BASE_URL}{path}`, sending the `query` params, and decodes the
/// JSON response. Must be called from within an async (tokio) context.
pub async fn get_json<T: DeserializeOwned>(
  path: &str,
  query: &[(String, String)],
) -> Result<T, GuiError> {
  let response = http_client()
    .get(format!("{API_BASE_URL}{path}"))
    .query(query)
    .send()
    .await
    .map_err(|e| GuiError::Api(e.to_string()))?
    .error_for_status()
    .map_err(|e| GuiError::Api(e.to_string()))?;
  response
    .json::<T>()
    .await
    .map_err(|e| GuiError::Api(e.to_string()))
}

/// Shared HTTP client (connection pooling).
fn http_client() -> &'static reqwest::Client {
  static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
  CLIENT.get_or_init(reqwest::Client::new)
}
