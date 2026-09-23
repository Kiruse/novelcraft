use std::sync::OnceLock;

use serde::de::DeserializeOwned;

use crate::error::GuiError;

// TODO: The base URL is hardcoded until the inspiration service gets a
// proper domain.

/// Base URL of the inspiration service.
pub(crate) const API_BASE_URL: &str = "http://localhost:8000";

/// Dedicated tokio runtime for HTTP requests — futures on the gpui
/// executor have no tokio context of their own.
pub(crate) fn http_runtime() -> &'static tokio::runtime::Runtime {
  static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
  RUNTIME.get_or_init(|| {
    tokio::runtime::Runtime::new().expect("Failed to initialize HTTP runtime")
  })
}

/// GETs `{API_BASE_URL}{path}`, sending the `query` params, and decodes the
/// JSON response.
pub(crate) async fn get_json<T: DeserializeOwned + Send + 'static>(
  path: &str,
  query: &[(String, String)],
) -> Result<T, GuiError> {
  let url = format!("{API_BASE_URL}{path}");
  let query: Vec<(String, String)> = query.to_vec();
  http_runtime()
    .spawn(async move {
      let response = http_client()
        .get(url)
        .query(&query)
        .send()
        .await
        .map_err(|e| GuiError::Api(e.to_string()))?
        .error_for_status()
        .map_err(|e| GuiError::Api(e.to_string()))?;
      response
        .json::<T>()
        .await
        .map_err(|e| GuiError::Api(e.to_string()))
    })
    .await
    .map_err(|e| GuiError::Api(e.to_string()))?
}

/// Shared HTTP client (connection pooling). Must be called from within the
/// tokio runtime context.
fn http_client() -> &'static reqwest::Client {
  static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
  CLIENT.get_or_init(reqwest::Client::new)
}
