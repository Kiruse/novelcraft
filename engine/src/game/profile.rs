use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::util::{serialize_timestamp, deserialize_timestamp};
use crate::util::prompting::Promptify;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileV1 {
  pub id: String,
  pub name: String,
  pub fields: HashMap<String, String>,
  #[serde(serialize_with = "serialize_timestamp", deserialize_with = "deserialize_timestamp")]
  pub created_at: DateTime<Utc>,
  #[serde(serialize_with = "serialize_timestamp", deserialize_with = "deserialize_timestamp")]
  pub updated_at: DateTime<Utc>,
}

impl ProfileV1 {
  pub fn new(id: String) -> Self {
    let now = Utc::now();
    Self {
      id,
      name: String::new(),
      fields: Default::default(),
      created_at: now.clone(),
      updated_at: now,
    }
  }

  pub fn with_name(self, name: String) -> Self {
    Self { name, updated_at: Utc::now(), ..self }
  }

  pub fn with_field(self, key: String, value: String) -> Self {
    Self {
      fields: Self::insert_field(self.fields, key, value),
      updated_at: Utc::now(),
      ..self
    }
  }

  pub fn with_fields(self, pairs: Vec<(String, String)>) -> Self {
    Self {
      fields: Self::insert_fields(self.fields, pairs),
      updated_at: Utc::now(),
      ..self
    }
  }

  fn insert_field(mut fields: HashMap<String, String>, key: String, value: String) -> HashMap<String, String> {
    fields.insert(key, value);
    fields
  }

  fn insert_fields(mut fields: HashMap<String, String>, pairs: Vec<(String, String)>) -> HashMap<String, String> {
    fields.extend(pairs);
    fields
  }
}

impl Promptify for ProfileV1 {
  fn promptify(&self, f: &mut crate::util::prompting::PromptFormatter) -> crate::util::prompting::PromptifyResult {
    f.write("Name: ")?;
    f.writeline(&self.name)?;
    self.fields.promptify(f)?;
    Ok(())
  }
}
