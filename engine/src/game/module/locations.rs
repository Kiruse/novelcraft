use std::collections::{BTreeMap, HashMap};

use kiruklaw_agent_loop::{AgentToolset, toolset};
use log::warn;
use serde::{Deserialize, Serialize};

use crate::error::EngineError;
use crate::game::module::GameStateView;
use crate::game::module::npcs::normalize_notes;
use crate::game::state::GameState;
use crate::markdown::BulletList;
use crate::util::prompting::{PromptFormatter, PromptifyResult};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocationsModule {}

impl LocationsModule {
  /// Read all locations tracked in this module's gamestate, keyed & ordered by name.
  async fn read(state: &GameState) -> Result<BTreeMap<String, LocationDescriptor>, EngineError> {
    let Some(module) = state.get(&Self::id()) else {
      return Ok(Default::default());
    };
    let locations: HashMap<String, LocationDescriptor> = module.get_all().await?;
    Ok(locations
      .into_iter()
      .filter(|(name, _)| !name.trim().is_empty())
      .map(|(name, mut location)| {
        location.name = name.clone();
        (name, location)
      })
      .collect())
  }

  pub async fn context(&self, state: GameState, f: &mut PromptFormatter) -> PromptifyResult {
    let locations = Self::read(&state)
      .await
      .map_err(|err| crate::util::prompting::PromptifyError::generic(err.to_string()))?;
    if locations.is_empty() {
      return Ok(());
    }

    f.writeline("## Locations")?;
    for (name, location) in &locations {
      if name.trim().is_empty() {
        warn!("Location missing name");
        continue;
      }

      f.write("### ")?;
      f.writeline(name)?;

      if !location.description.trim().is_empty() {
        f.write("Description: ")?;
        f.writeline(&location.description)?;
      }

      if !location.notes.trim().is_empty() {
        f.writeline("Notes:")?;
        f.writeline(location.notes.trim())?;
      }
    }

    Ok(())
  }
}

#[toolset(strip_suffix = "module")]
impl AgentToolset<GameStateView> for LocationsModule {
  /// Add a new location to the story or refine an existing one. Locations
  /// are singular places of interest — buildings, rooms, landmarks, towns.
  /// Only non-empty values are written — already known details are never
  /// erased by omitting them. Call this whenever the players visit a new
  /// noteworthy place or you learn/refine something about an existing one.
  /// @name Name of the location to add or update.
  /// @description Appearance & atmosphere of this location. Leave empty to keep the currently known description.
  async fn upsert_location(
    &self,
    state: GameStateView,
    name: String,
    description: String,
  ) -> Result<String, anyhow::Error> {
    let name = name.trim().to_string();
    if name.is_empty() {
      return Err(EngineError::validation("location name must not be empty").into());
    }

    let description = description.trim();
    if description.is_empty() {
      let existed = state
        .get::<serde_json::Value>(&Self::ID.to_string(), std::slice::from_ref(&name))
        .await
        .transpose()?
        .is_some();
      if existed {
        return Ok(format!("Location {name} keeps its description."));
      }
      return Err(EngineError::validation(format!(
        "no description provided for location {name}"
      )).into());
    }

    let existed = state
      .get::<serde_json::Value>(&Self::ID.to_string(), std::slice::from_ref(&name))
      .await
      .transpose()?
      .is_some();
    state.set(&[name.clone(), "description".to_string()], &description).await?;

    if existed {
      Ok(format!("Location {name} updated."))
    } else {
      Ok(format!("Location {name} added to the story."))
    }
  }

  /// Update the named location's description, replacing the known one.
  /// Use this when a location changes in some relevant way.
  /// @name Name of the location whose description to update.
  /// @description The location's new appearance & atmosphere.
  async fn update_description(
    &self,
    state: GameStateView,
    name: String,
    description: String,
  ) -> Result<String, anyhow::Error> {
    let name = name.trim().to_string();
    if name.is_empty() {
      return Err(EngineError::validation("location name must not be empty").into());
    }
    let description = description.trim();
    if description.is_empty() {
      return Err(EngineError::validation("description must not be empty").into());
    }

    state
      .set(&Keypath::Description(name.clone()).into_keypath(), &description)
      .await?;
    Ok(format!("Description of {name} successfully updated."))
  }

  /// Add a concise bullet point to the named location's notes. Use this to
  /// record noteworthy facts about the place as the story reveals them.
  /// @name Name of the location to add a note to.
  /// @note The concise note to add. Multiple notes may be separated by newlines; a leading "- " is optional.
  async fn add_note(
    &self,
    state: GameStateView,
    name: String,
    note: String,
  ) -> Result<String, anyhow::Error> {
    let name = name.trim().to_string();
    if name.is_empty() {
      return Err(EngineError::validation("location name must not be empty").into());
    }
    let items = normalize_notes(&note);
    if items.is_empty() {
      return Err(EngineError::validation("note must not be empty").into());
    }

    let keypath = Keypath::Notes(name.clone()).into_keypath();
    let existing = state
      .get::<String>(&Self::ID.to_string(), &keypath)
      .await
      .transpose()?
      .unwrap_or_default();
    let mut list = BulletList::parse(&existing)?;
    list.0.extend(items);

    state.set(&keypath, &list.to_string()).await?;
    Ok(format!("Note added to {name}."))
  }

  /// Remove a bullet point from the named location's notes, matching its
  /// text (case-insensitive).
  /// @name Name of the location to remove a note from.
  /// @note The note to remove, matching the bullet's text. Multiple notes may be separated by newlines; a leading "- " is optional.
  async fn remove_note(
    &self,
    state: GameStateView,
    name: String,
    note: String,
  ) -> Result<String, anyhow::Error> {
    let name = name.trim().to_string();
    if name.is_empty() {
      return Err(EngineError::validation("location name must not be empty").into());
    }
    let targets: Vec<String> =
      normalize_notes(&note).into_iter().map(|t| t.to_lowercase()).collect();
    if targets.is_empty() {
      return Err(EngineError::validation("note must not be empty").into());
    }

    let keypath = Keypath::Notes(name.clone()).into_keypath();
    let existing = state
      .get::<String>(&Self::ID.to_string(), &keypath)
      .await
      .transpose()?
      .unwrap_or_default();
    let list = BulletList::parse(&existing)?;
    let removed = list.len();

    let remaining: Vec<String> = list
      .0
      .into_iter()
      .filter(|item| !targets.contains(&item.trim().to_lowercase()))
      .collect();
    let removed = removed - remaining.len();
    if removed == 0 {
      return Ok(format!("No matching note found for {name}."));
    }

    state.set(&keypath, &BulletList(remaining).to_string()).await?;
    Ok(format!("Removed {removed} note(s) from {name}."))
  }

  /// Rename an existing location, moving all of its known details to the
  /// new name.
  /// @old_name Current name of the location.
  /// @new_name The location's new name.
  async fn rename_location(
    &self,
    state: GameStateView,
    old_name: String,
    new_name: String,
  ) -> Result<String, anyhow::Error> {
    let old_name = old_name.trim().to_string();
    let new_name = new_name.trim().to_string();
    if old_name.is_empty() || new_name.is_empty() {
      return Err(EngineError::validation("location names must not be empty").into());
    }
    if old_name == new_name {
      return Ok(format!("Location {old_name} keeps its name."));
    }

    let Some(descriptor) = state
      .get::<serde_json::Value>(&Self::ID.to_string(), std::slice::from_ref(&old_name))
      .await
      .transpose()?
    else {
      return Err(EngineError::not_found(format!("no location named {old_name}")).into());
    };
    if state
      .get::<serde_json::Value>(&Self::ID.to_string(), std::slice::from_ref(&new_name))
      .await
      .transpose()?
      .is_some()
    {
      return Err(EngineError::validation(format!("a location named {new_name} already exists")).into());
    }

    state.set(std::slice::from_ref(&new_name), &descriptor).await?;
    state.remove(std::slice::from_ref(&old_name)).await?;
    Ok(format!("Location {old_name} renamed to {new_name}."))
  }
}

impl From<LocationsModule> for super::GameplayModule {
  fn from(value: LocationsModule) -> Self {
    Self::Locations(value)
  }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocationDescriptor {
  /// Name of this location.
  #[serde(default)]
  pub name: String,
  /// Appearance & atmosphere of this location.
  #[serde(default)]
  pub description: String,
  /// Concise bullet points of noteworthy facts about this location.
  #[serde(default)]
  pub notes: String,
}

#[derive(Debug, Clone)]
enum Keypath {
  Description(String),
  Notes(String),
}

impl Keypath {
  fn into_keypath(self) -> Vec<String> {
    match self {
      Self::Description(name) => vec![name, "description".to_string()],
      Self::Notes(name) => vec![name, "notes".to_string()],
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::game::state::GameState;

  fn state() -> GameState {
    GameState::new(Default::default(), vec![LocationsModule::ID.to_string()], None).unwrap()
  }

  #[tokio::test]
  async fn upsert_creates_then_refines() {
    let state = state();
    let view = state.view(LocationsModule::ID.to_string());
    let module = LocationsModule::default();

    let msg = module
      .upsert_location(
        view.clone(),
        "The Rusty Flagon".to_string(),
        "a smoke-filled tavern by the docks".to_string(),
      )
      .await
      .unwrap();
    assert!(msg.contains("added"));

    let locations = LocationsModule::read(&state).await.unwrap();
    assert_eq!(
      locations.get("The Rusty Flagon").unwrap().description,
      "a smoke-filled tavern by the docks"
    );

    // An empty description keeps the current one.
    let msg = module
      .upsert_location(view, "The Rusty Flagon".to_string(), String::new())
      .await
      .unwrap();
    assert!(msg.contains("keeps its description"));
    assert_eq!(
      LocationsModule::read(&state).await.unwrap()["The Rusty Flagon"].description,
      "a smoke-filled tavern by the docks"
    );
  }

  #[tokio::test]
  async fn upsert_validates() {
    let module = LocationsModule::default();
    let view = state().view(LocationsModule::ID.to_string());
    // Empty name:
    assert!(
      module
        .upsert_location(view.clone(), "  ".to_string(), "somewhere".to_string())
        .await
        .is_err()
    );
    // No description for a new location:
    assert!(
      module
        .upsert_location(view, "Nowhere".to_string(), String::new())
        .await
        .is_err()
    );
    assert!(LocationsModule::read(&state()).await.unwrap().is_empty());
  }

  #[tokio::test]
  async fn update_description_overwrites() {
    let state = state();
    let view = state.view(LocationsModule::ID.to_string());
    let module = LocationsModule::default();

    module
      .upsert_location(view.clone(), "Market".to_string(), "bustling stalls".to_string())
      .await
      .unwrap();
    module
      .update_description(view, "Market".to_string(), "burnt ruins".to_string())
      .await
      .unwrap();

    assert_eq!(
      LocationsModule::read(&state).await.unwrap()["Market"].description,
      "burnt ruins"
    );
  }

  #[tokio::test]
  async fn rename_location_moves_all_details() {
    let state = state();
    let view = state.view(LocationsModule::ID.to_string());
    let module = LocationsModule::default();

    module
      .upsert_location(view.clone(), "Old Keep".to_string(), "crumbling walls".to_string())
      .await
      .unwrap();
    module
      .add_note(view.clone(), "Old Keep".to_string(), "haunted since the siege".to_string())
      .await
      .unwrap();

    let msg = module
      .rename_location(view.clone(), "Old Keep".to_string(), "Ravencrest Keep".to_string())
      .await
      .unwrap();
    assert!(msg.contains("renamed"));

    let locations = LocationsModule::read(&state).await.unwrap();
    assert!(locations.get("Old Keep").is_none());
    let keep = locations.get("Ravencrest Keep").unwrap();
    assert_eq!(keep.description, "crumbling walls");
    assert!(keep.notes.contains("haunted since the siege"));
  }

  #[tokio::test]
  async fn rename_location_validates() {
    let state = state();
    let view = state.view(LocationsModule::ID.to_string());
    let module = LocationsModule::default();
    module
      .upsert_location(view.clone(), "Keep".to_string(), "old".to_string())
      .await
      .unwrap();
    module
      .upsert_location(view.clone(), "Mill".to_string(), "riverside".to_string())
      .await
      .unwrap();

    // Unknown old name:
    assert!(
      module
        .rename_location(view.clone(), "Nowhere".to_string(), "Somewhere".to_string())
        .await
        .is_err()
    );
    // New name already taken:
    assert!(
      module
        .rename_location(view.clone(), "Keep".to_string(), "Mill".to_string())
        .await
        .is_err()
    );
    // Empty names:
    assert!(
      module
        .rename_location(view.clone(), String::new(), "Somewhere".to_string())
        .await
        .is_err()
    );
    // A same-name rename is a friendly no-op:
    assert!(module.rename_location(view, "Keep".to_string(), "Keep".to_string()).await.is_ok());

    let locations = LocationsModule::read(&state).await.unwrap();
    assert_eq!(locations.len(), 2);
  }

  #[tokio::test]
  async fn add_and_remove_notes() {
    let state = state();
    let view = state.view(LocationsModule::ID.to_string());
    let module = LocationsModule::default();

    module
      .add_note(view.clone(), "Mill".to_string(), "the miller owes a debt".to_string())
      .await
      .unwrap();
    // Prefixes & extra whitespace are normalized away.
    module
      .add_note(view.clone(), "Mill".to_string(), "- rats in the cellar ".to_string())
      .await
      .unwrap();

    assert_eq!(
      LocationsModule::read(&state).await.unwrap()["Mill"].notes,
      "- the miller owes a debt\n- rats in the cellar\n"
    );

    // Removal matches case-insensitively, prefixes optional.
    let msg = module
      .remove_note(view, "Mill".to_string(), "- Rats in the Cellar".to_string())
      .await
      .unwrap();
    assert!(msg.contains("Removed 1"));
    assert_eq!(
      LocationsModule::read(&state).await.unwrap()["Mill"].notes,
      "- the miller owes a debt\n"
    );
  }

  #[tokio::test]
  async fn context_renders_locations() {
    let state = state();
    let view = state.view(LocationsModule::ID.to_string());
    let module = LocationsModule::default();
    module
      .upsert_location(view.clone(), "Mill".to_string(), "riverside mill".to_string())
      .await
      .unwrap();
    module
      .add_note(view, "Mill".to_string(), "sells questionable flour".to_string())
      .await
      .unwrap();

    let mut f = PromptFormatter::new();
    module.context(state, &mut f).await.unwrap();
    let text = f.finish();

    assert!(text.contains("## Locations"));
    assert!(text.contains("### Mill"));
    assert!(text.contains("Description: riverside mill"));
    assert!(text.contains("- sells questionable flour"));
  }

  #[tokio::test]
  async fn context_is_empty_without_locations() {
    let mut f = PromptFormatter::new();
    LocationsModule::default().context(state(), &mut f).await.unwrap();
    assert!(f.finish().trim().is_empty());
  }
}
