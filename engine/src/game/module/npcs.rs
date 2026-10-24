use std::collections::{BTreeMap, HashMap};

use kiruklaw_agent_loop::{AgentToolset, toolset};
use log::warn;
use serde::{Deserialize, Serialize};

use crate::error::EngineError;
use crate::game::module::GameStateView;
use crate::game::state::GameState;
use crate::markdown::BulletList;
use crate::util::prompting::{PromptFormatter, PromptifyResult};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NpcsModule {}

impl NpcsModule {
  /// Read all NPCs tracked in this module's gamestate, keyed & ordered by name.
  async fn read(state: &GameState) -> Result<BTreeMap<String, NpcDescriptor>, EngineError> {
    let Some(module) = state.get(&Self::id()) else {
      return Ok(Default::default());
    };
    let npcs: HashMap<String, NpcDescriptor> = module.get_all().await?;
    Ok(npcs
      .into_iter()
      .filter(|(name, _)| !name.trim().is_empty())
      .map(|(name, mut npc)| {
        npc.name = name.clone();
        (name, npc)
      })
      .collect())
  }

  pub async fn context(&self, state: GameState, f: &mut PromptFormatter) -> PromptifyResult {
    let npcs = Self::read(&state)
      .await
      .map_err(|err| crate::util::prompting::PromptifyError::generic(err.to_string()))?;
    if npcs.is_empty() {
      return Ok(());
    }

    f.writeline("## NPCs")?;
    for (name, npc) in &npcs {
      if name.trim().is_empty() {
        warn!("NPC missing name");
        continue;
      }

      f.write("### ")?;
      f.writeline(name)?;

      if !npc.personality.trim().is_empty() {
        f.write("Personality: ")?;
        f.writeline(&npc.personality)?;
      }

      if !npc.physique.trim().is_empty() {
        f.write("Physique: ")?;
        f.writeline(&npc.physique)?;
      }

      if !npc.apparel.trim().is_empty() {
        f.write("Apparel: ")?;
        f.writeline(&npc.apparel)?;
      }

      if !npc.notes.trim().is_empty() {
        f.writeline("Notes:")?;
        f.writeline(npc.notes.trim())?;
      }
    }

    Ok(())
  }
}

/// Normalize raw tool input into note items: trimmed, `- ` prefixes and
/// bare dashes stripped, empty lines dropped.
pub(crate) fn normalize_notes(note: &str) -> Vec<String> {
  note
    .lines()
    .map(str::trim)
    .filter(|line| !line.is_empty())
    .map(|line| line.strip_prefix("- ").map(str::trim).unwrap_or(line))
    .filter(|line| !line.is_empty() && *line != "-")
    .map(str::to_string)
    .collect()
}

#[toolset(strip_suffix = "module")]
impl AgentToolset<GameStateView> for NpcsModule {
  /// Add a new NPC to the story or refine an existing one. NPCs are
  /// singular characters only — never register groups, pairs or crowds;
  /// each noteworthy character gets their own entry. Only fields with
  /// non-empty values are written — already known details are never erased
  /// by omitting them. Call this whenever a new character appears or you
  /// learn/refine something about an existing one.
  /// @name Full name of the character to add or update.
  /// @personality Personality of this character, including favorites & despised. Leave empty to keep the currently known personality.
  /// @physique Physique of this character, excluding apparel & equipment. Leave empty to keep the currently known physique.
  /// @apparel Apparel and/or equipment of this character, as comma-separated list preferably. Leave empty to keep the currently known apparel.
  async fn upsert_npc(
    &self,
    state: GameStateView,
    name: String,
    personality: String,
    physique: String,
    apparel: String,
  ) -> Result<String, anyhow::Error> {
    let name = name.trim().to_string();
    if name.is_empty() {
      return Err(EngineError::validation("npc name must not be empty").into());
    }

    let existed = state
      .get::<serde_json::Value>(&Self::ID.to_string(), std::slice::from_ref(&name))
      .await
      .transpose()?
      .is_some();

    let mut provided = false;
    for (field, value) in [
      ("personality", personality),
      ("physique", physique),
      ("apparel", apparel),
    ] {
      let value = value.trim();
      if value.is_empty() {
        continue;
      }
      provided = true;
      state.set(&[name.clone(), field.to_string()], &value).await?;
    }

    if !provided {
      return Err(EngineError::validation(format!(
        "no details provided for npc {name}: at least one of personality, physique or apparel is required"
      )).into());
    }

    if existed {
      Ok(format!("Npc {name} updated."))
    } else {
      Ok(format!("Npc {name} added to the story."))
    }
  }

  /// Add a concise bullet point to the named NPC's notes. Use this to
  /// record noteworthy facts about the character as the story reveals
  /// them.
  /// @name Name of the NPC to add a note to.
  /// @note The concise note to add. Multiple notes may be separated by newlines; a leading "- " is optional.
  async fn add_note(
    &self,
    state: GameStateView,
    name: String,
    note: String,
  ) -> Result<String, anyhow::Error> {
    let name = name.trim().to_string();
    if name.is_empty() {
      return Err(EngineError::validation("npc name must not be empty").into());
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

  /// Remove a bullet point from the named NPC's notes, matching its text
  /// (case-insensitive).
  /// @name Name of the NPC to remove a note from.
  /// @note The note to remove, matching the bullet's text. Multiple notes may be separated by newlines; a leading "- " is optional.
  async fn remove_note(
    &self,
    state: GameStateView,
    name: String,
    note: String,
  ) -> Result<String, anyhow::Error> {
    let name = name.trim().to_string();
    if name.is_empty() {
      return Err(EngineError::validation("npc name must not be empty").into());
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

  /// Rename an existing NPC, moving all of their known details to the new
  /// name. Use this once a placeholder-named character reveals their actual
  /// name.
  /// @old_name Current name of the NPC.
  /// @new_name The NPC's new full name.
  async fn rename_npc(
    &self,
    state: GameStateView,
    old_name: String,
    new_name: String,
  ) -> Result<String, anyhow::Error> {
    let old_name = old_name.trim().to_string();
    let new_name = new_name.trim().to_string();
    if old_name.is_empty() || new_name.is_empty() {
      return Err(EngineError::validation("npc names must not be empty").into());
    }
    if old_name == new_name {
      return Ok(format!("Npc {old_name} keeps their name."));
    }

    let Some(descriptor) = state
      .get::<serde_json::Value>(&Self::ID.to_string(), std::slice::from_ref(&old_name))
      .await
      .transpose()?
    else {
      return Err(EngineError::not_found(format!("no npc named {old_name}")).into());
    };
    if state
      .get::<serde_json::Value>(&Self::ID.to_string(), std::slice::from_ref(&new_name))
      .await
      .transpose()?
      .is_some()
    {
      return Err(EngineError::validation(format!("an npc named {new_name} already exists")).into());
    }

    state.set(std::slice::from_ref(&new_name), &descriptor).await?;
    state.remove(std::slice::from_ref(&old_name)).await?;
    Ok(format!("Npc {old_name} renamed to {new_name}."))
  }

  /// Update the named NPC's apparel.
  /// @name Name of the NPC whose apparel to update.
  /// @apparel Comma-separated list of apparel items.
  async fn update_apparel(
    &self,
    state: GameStateView,
    name: String,
    apparel: String,
  ) -> Result<String, anyhow::Error> {
    state.set(&Keypath::Apparel(name.clone()).into_keypath(), &apparel).await?;
    Ok(format!("Apparel of {name} successfully updated."))
  }
}

impl From<NpcsModule> for super::GameplayModule {
  fn from(value: NpcsModule) -> Self {
    Self::Npcs(value)
  }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NpcDescriptor {
  /// Full name of this character.
  #[serde(default)]
  pub name: String,
  /// Personality of this character, including favorites & despised.
  #[serde(default)]
  pub personality: String,
  /// Physique of this character, excluding apparel & equipment.
  #[serde(default)]
  pub physique: String,
  /// Apparel and/or equipment of this character, as comma-separated list preferably.
  #[serde(default)]
  pub apparel: String,
  /// Concise bullet points of noteworthy facts about this character.
  #[serde(default)]
  pub notes: String,
}

#[derive(Debug, Clone)]
enum Keypath {
  Apparel(String),
  Notes(String),
}

impl Keypath {
  fn into_keypath(self) -> Vec<String> {
    match self {
      Self::Apparel(name) => vec![name, "apparel".to_string()],
      Self::Notes(name) => vec![name, "notes".to_string()],
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::game::state::GameState;

  fn state() -> GameState {
    GameState::new(Default::default(), vec![NpcsModule::ID.to_string()], None).unwrap()
  }

  #[tokio::test]
  async fn upsert_creates_then_refines() {
    let state = state();
    let view = state.view(NpcsModule::ID.to_string());
    let module = NpcsModule::default();

    let msg = module
      .upsert_npc(
        view.clone(),
        "Mira".to_string(),
        "warm & wry".to_string(),
        String::new(),
        "travel cloak".to_string(),
      )
      .await
      .unwrap();
    assert!(msg.contains("added"));

    let npcs = NpcsModule::read(&state).await.unwrap();
    let mira = npcs.get("Mira").unwrap();
    assert_eq!(mira.personality, "warm & wry");
    assert_eq!(mira.physique, "");
    assert_eq!(mira.apparel, "travel cloak");

    // Refining with only a physique preserves the other fields.
    let msg = module
      .upsert_npc(
        view,
        "Mira".to_string(),
        String::new(),
        "tall & wiry".to_string(),
        String::new(),
      )
      .await
      .unwrap();
    assert!(msg.contains("updated"));

    let npcs = NpcsModule::read(&state).await.unwrap();
    let mira = npcs.get("Mira").unwrap();
    assert_eq!(mira.personality, "warm & wry");
    assert_eq!(mira.physique, "tall & wiry");
    assert_eq!(mira.apparel, "travel cloak");
  }

  #[tokio::test]
  async fn upsert_rejects_empty_name() {
    let module = NpcsModule::default();
    let view = state().view(NpcsModule::ID.to_string());
    assert!(
      module
        .upsert_npc(view, "  ".to_string(), "x".to_string(), String::new(), String::new())
        .await
        .is_err()
    );
  }

  #[tokio::test]
  async fn rename_npc_moves_all_details() {
    let state = state();
    let view = state.view(NpcsModule::ID.to_string());
    let module = NpcsModule::default();

    module
      .upsert_npc(
        view.clone(),
        "Blonde Student".to_string(),
        "studious".to_string(),
        String::new(),
        "college outfit".to_string(),
      )
      .await
      .unwrap();
    module
      .add_note(view.clone(), "Blonde Student".to_string(), "sits by the window".to_string())
      .await
      .unwrap();

    let msg = module
      .rename_npc(view.clone(), "Blonde Student".to_string(), "Clara Voss".to_string())
      .await
      .unwrap();
    assert!(msg.contains("renamed"));

    let npcs = NpcsModule::read(&state).await.unwrap();
    assert!(npcs.get("Blonde Student").is_none());
    let clara = npcs.get("Clara Voss").unwrap();
    assert_eq!(clara.personality, "studious");
    assert_eq!(clara.apparel, "college outfit");
    assert!(clara.notes.contains("sits by the window"));
  }

  #[tokio::test]
  async fn rename_npc_validates() {
    let state = state();
    let view = state.view(NpcsModule::ID.to_string());
    let module = NpcsModule::default();
    module
      .upsert_npc(view.clone(), "Mira".to_string(), "wry".to_string(), String::new(), String::new())
      .await
      .unwrap();
    module
      .upsert_npc(view.clone(), "Bo".to_string(), "quiet".to_string(), String::new(), String::new())
      .await
      .unwrap();

    // Unknown old name:
    assert!(
      module
        .rename_npc(view.clone(), "Nobody".to_string(), "Someone".to_string())
        .await
        .is_err()
    );
    // New name already taken:
    assert!(
      module
        .rename_npc(view.clone(), "Mira".to_string(), "Bo".to_string())
        .await
        .is_err()
    );
    // Empty names:
    assert!(
      module
        .rename_npc(view.clone(), String::new(), "Someone".to_string())
        .await
        .is_err()
    );
    // A same-name rename is a friendly no-op:
    assert!(
      module
        .rename_npc(view, "Mira".to_string(), "Mira".to_string())
        .await
        .is_ok()
    );

    // Nothing changed overall:
    let npcs = NpcsModule::read(&state).await.unwrap();
    assert_eq!(npcs.len(), 2);
    assert!(npcs.contains_key("Mira"));
    assert!(npcs.contains_key("Bo"));
  }

  #[tokio::test]
  async fn upsert_rejects_fully_empty() {
    let module = NpcsModule::default();
    let view = state().view(NpcsModule::ID.to_string());
    assert!(
      module
        .upsert_npc(view, "Mira".to_string(), String::new(), String::new(), String::new())
        .await
        .is_err()
    );
    assert!(NpcsModule::read(&state()).await.unwrap().is_empty());
  }

  #[tokio::test]
  async fn context_renders_npcs_from_state() {
    let state = state();
    let view = state.view(NpcsModule::ID.to_string());
    NpcsModule::default()
      .update_apparel(view, "Bo".to_string(), "leather vest".to_string())
      .await
      .unwrap();

    let mut f = PromptFormatter::new();
    NpcsModule::default().context(state, &mut f).await.unwrap();
    let text = f.finish();

    assert!(text.contains("## NPCs"));
    assert!(text.contains("### Bo"));
    assert!(text.contains("Apparel: leather vest"));
  }

  #[tokio::test]
  async fn add_and_remove_notes() {
    let state = state();
    let view = state.view(NpcsModule::ID.to_string());
    let module = NpcsModule::default();

    module
      .add_note(view.clone(), "Mira".to_string(), "owes a debt to the guild".to_string())
      .await
      .unwrap();
    // Prefixes & extra whitespace are normalized away.
    module
      .add_note(view.clone(), "Mira".to_string(), "- afraid of dogs ".to_string())
      .await
      .unwrap();

    let npcs = NpcsModule::read(&state).await.unwrap();
    assert_eq!(
      npcs.get("Mira").unwrap().notes,
      "- owes a debt to the guild\n- afraid of dogs\n"
    );

    // Removal matches case-insensitively, prefixes optional.
    let msg = module
      .remove_note(view, "Mira".to_string(), "- Afraid of Dogs".to_string())
      .await
      .unwrap();
    assert!(msg.contains("Removed 1"));

    let npcs = NpcsModule::read(&state).await.unwrap();
    assert_eq!(npcs.get("Mira").unwrap().notes, "- owes a debt to the guild\n");
  }

  #[tokio::test]
  async fn remove_note_without_match_is_soft() {
    let state = state();
    let view = state.view(NpcsModule::ID.to_string());
    let module = NpcsModule::default();
    module
      .add_note(view.clone(), "Mira".to_string(), "owes a debt".to_string())
      .await
      .unwrap();

    let msg = module
      .remove_note(view, "Mira".to_string(), "killed a man".to_string())
      .await
      .unwrap();
    assert!(msg.contains("No matching note"));

    let npcs = NpcsModule::read(&state).await.unwrap();
    assert_eq!(npcs.get("Mira").unwrap().notes, "- owes a debt\n");
  }

  #[tokio::test]
  async fn add_note_rejects_empty() {
    let module = NpcsModule::default();
    let view = state().view(NpcsModule::ID.to_string());
    assert!(module.add_note(view.clone(), "Mira".to_string(), String::new()).await.is_err());
    assert!(module.add_note(view, "Mira".to_string(), "  \n-  \n".to_string()).await.is_err());
  }

  #[tokio::test]
  async fn context_renders_notes() {
    let state = state();
    let view = state.view(NpcsModule::ID.to_string());
    let module = NpcsModule::default();
    module
      .add_note(view, "Bo".to_string(), "sells questionable maps".to_string())
      .await
      .unwrap();

    let mut f = PromptFormatter::new();
    module.context(state, &mut f).await.unwrap();
    let text = f.finish();

    assert!(text.contains("Notes:"));
    assert!(text.contains("- sells questionable maps"));
  }

  #[tokio::test]
  async fn context_is_empty_without_npcs() {
    let mut f = PromptFormatter::new();
    NpcsModule::default().context(state(), &mut f).await.unwrap();
    assert!(f.finish().trim().is_empty());
  }
}
