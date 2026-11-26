pub use serde_json::{json, Map as JsonMap, Value as JsonValue};

pub trait SerdeJsonExt {
  fn get_str(&self, field: &str) -> Option<&str>;
  fn get_obj(&self, field: &str) -> Option<&JsonMap<String, JsonValue>>;
  fn get_obj_mut(&mut self, field: &str) -> Option<&mut JsonMap<String, JsonValue>>;
}

impl SerdeJsonExt for JsonValue {
  fn get_str(&self, field: &str) -> Option<&str> {
    self.get(field).and_then(|v| v.as_str())
  }

  fn get_obj(&self, field: &str) -> Option<&JsonMap<String, JsonValue>> {
    self.get(field).and_then(|v| v.as_object())
  }

  fn get_obj_mut(&mut self, field: &str) -> Option<&mut JsonMap<String, JsonValue>> {
    self.get_mut(field).and_then(|v| v.as_object_mut())
  }
}
