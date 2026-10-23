//! OPFS-backed file primitives for the wasm32 target (browser storage).
//!
//! Paths are logical, relative to the OPFS root (see `crate::paths`): the
//! components address nested directories, the last component of a file path
//! names the file. All operations are async — the main thread has no
//! synchronous OPFS access.

use std::path::Path;

use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

use crate::error::EngineError;

type DirHandle = web_sys::FileSystemDirectoryHandle;
type FileHandle = web_sys::FileSystemFileHandle;

/// Render a JS rejection as an engine I/O error.
fn js_error(context: &str, err: JsValue) -> EngineError {
  let name = js_sys::Reflect::get(&err, &JsValue::from_str("name"))
    .ok()
    .and_then(|value| value.as_string());
  let message = js_sys::Reflect::get(&err, &JsValue::from_str("message"))
    .ok()
    .and_then(|value| value.as_string());
  match (name, message) {
    (Some(name), Some(message)) => EngineError::io(format!("{context}: {name}: {message}")),
    _ => EngineError::io(format!("{context}: {err:?}")),
  }
}

/// Whether a JS rejection is a filesystem "not found" miss.
fn is_not_found(err: &JsValue) -> bool {
  js_sys::Reflect::get(err, &JsValue::from_str("name"))
    .ok()
    .and_then(|value| value.as_string())
    .as_deref() == Some("NotFoundError")
}

/// Await a promise, rendering rejections as engine errors.
async fn await_promise(promise: js_sys::Promise, context: &str) -> Result<JsValue, EngineError> {
  wasm_bindgen_futures::JsFuture::from(promise)
    .await
    .map_err(|err| js_error(context, err))
}

/// Await a lookup promise, yielding `None` on a "not found" rejection.
async fn await_optional(promise: js_sys::Promise, context: &str) -> Result<Option<JsValue>, EngineError> {
  match wasm_bindgen_futures::JsFuture::from(promise).await {
    Ok(value) => Ok(Some(value)),
    Err(err) if is_not_found(&err) => Ok(None),
    Err(err) => Err(js_error(context, err)),
  }
}

/// The OPFS root directory handle.
async fn root() -> Result<DirHandle, EngineError> {
  let storage = web_sys::window()
    .ok_or_else(|| EngineError::internal("no global window object"))?
    .navigator()
    .storage();
  await_promise(storage.get_directory(), "getDirectory")
    .await?
    .dyn_into()
    .map_err(|_| EngineError::internal("OPFS root is not a directory handle"))
}

/// Walk (and optionally create) the directory chain of a logical path.
/// Returns `None` when `create` is off and the path does not exist.
async fn dir_handle(path: &Path, create: bool) -> Result<Option<DirHandle>, EngineError> {
  let mut dir = root().await?;
  for component in path.components() {
    let name = component.as_os_str().to_string_lossy().to_string();
    if name.is_empty() || name == "." {
      continue;
    }
    let mut options = web_sys::FileSystemGetDirectoryOptions::new();
    let options = options.create(create);
    match await_optional(dir.get_directory_handle_with_options(&name, &options), "getDirectoryHandle").await? {
      Some(value) => {
        dir = value
          .dyn_into()
          .map_err(|_| EngineError::io(format!("getDirectoryHandle: {name:?} is not a directory")))?;
      }
      None => return Ok(None),
    }
  }
  Ok(Some(dir))
}

/// Resolve the file handle of a logical path. Returns `None` when `create`
/// is off and the file (or any of its parents) does not exist.
async fn file_handle(path: &Path, create: bool) -> Result<Option<FileHandle>, EngineError> {
  let Some(name) = path.file_name().map(|name| name.to_string_lossy().to_string()) else {
    return Err(EngineError::internal("path has no file name"));
  };
  let parent = path.parent().unwrap_or_else(|| Path::new(""));
  let Some(dir) = dir_handle(parent, false).await? else {
    return Ok(None);
  };
  let mut options = web_sys::FileSystemGetFileOptions::new();
  let options = options.create(create);
  match await_optional(dir.get_file_handle_with_options(&name, &options), "getFileHandle").await? {
    Some(value) => Ok(Some(
      value
        .dyn_into()
        .map_err(|_| EngineError::io(format!("getFileHandle: {name:?} is not a file")))?,
    )),
    None => Ok(None),
  }
}

/// Create a directory and all of its parents.
pub(crate) async fn ensure_dir(path: &Path) -> Result<(), EngineError> {
  dir_handle(path, true).await.map(|_| ())
}

/// Read the whole text content of a file.
pub(crate) async fn read(path: &Path) -> Result<String, EngineError> {
  let Some(handle) = file_handle(path, false).await? else {
    return Err(EngineError::io("Read error: file does not exist"));
  };
  let file: web_sys::File = await_promise(handle.get_file(), "getFile")
    .await?
    .dyn_into()
    .map_err(|_| EngineError::internal("getFile did not resolve to a File"))?;
  await_promise(file.text(), "text")
    .await?
    .as_string()
    .ok_or_else(|| EngineError::internal("file text is not a string"))
}

/// Write (or create) a file with the given text content.
pub(crate) async fn write(path: &Path, content: &str) -> Result<(), EngineError> {
  let Some(handle) = file_handle(path, true).await? else {
    return Err(EngineError::io("Write error: could not create file"));
  };
  let writable: web_sys::FileSystemWritableFileStream = await_promise(handle.create_writable(), "createWritable")
    .await?
    .dyn_into()
    .map_err(|_| EngineError::internal("createWritable did not resolve to a writable stream"))?;
  let promise = writable
    .write_with_str(content)
    .map_err(|err| js_error("write", err))?;
  await_promise(promise, "write").await?;
  await_promise(writable.close(), "close").await?;
  Ok(())
}

/// Remove a file; a missing file is a no-op.
pub(crate) async fn remove_file(path: &Path) -> Result<(), EngineError> {
  let Some(name) = path.file_name().map(|name| name.to_string_lossy().to_string()) else {
    return Err(EngineError::internal("path has no file name"));
  };
  let parent = path.parent().unwrap_or_else(|| Path::new(""));
  let Some(dir) = dir_handle(parent, false).await? else {
    return Ok(());
  };
  await_optional(dir.remove_entry(&name), "removeEntry").await?;
  Ok(())
}

/// List a directory's entries as `(name, is_directory)`. A missing
/// directory yields an empty list.
pub(crate) async fn list_dir(path: &Path) -> Result<Vec<(String, bool)>, EngineError> {
  let Some(dir) = dir_handle(path, false).await? else {
    return Ok(Vec::new());
  };
  let iterator = dir.values();
  let mut entries = Vec::new();
  loop {
    let promise = iterator.next().map_err(|err| js_error("values", err))?;
    let entry = await_promise(promise, "values").await?;
    let done = js_sys::Reflect::get(&entry, &JsValue::from_str("done"))
      .map_err(|err| js_error("values", err))?
      .as_bool()
      .unwrap_or(true);
    if done {
      break;
    }
    let value = js_sys::Reflect::get(&entry, &JsValue::from_str("value"))
      .map_err(|err| js_error("values", err))?;
    let Some(name) = js_sys::Reflect::get(&value, &JsValue::from_str("0"))
      .ok()
      .and_then(|name| name.as_string()) else {
      continue;
    };
    let kind = js_sys::Reflect::get(&value, &JsValue::from_str("1"))
      .ok()
      .and_then(|handle| js_sys::Reflect::get(&handle, &JsValue::from_str("kind")).ok())
      .and_then(|kind| kind.as_string())
      .unwrap_or_default();
    entries.push((name, kind == "directory"));
  }
  Ok(entries)
}
