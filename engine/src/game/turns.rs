use std::collections::HashSet;

use kiruklaw_agent_loop::{Conversation, ConversationMessage, ToolCall};
use serde::{Deserialize, Serialize};

use crate::error::EngineError;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TurnV2 {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub system: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub prompt: Option<String>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub responses: Vec<AgentResponse>,
  /// Manual gamestate edits made by the player from the sidebar while this
  /// was the last turn. Each pair is a full keypath (first segment = module
  /// ID) and its string override; an empty string removes the field. Applied
  /// after the turn's tool calls during [`crate::game::state::GameState::replay`].
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub edits: Vec<(Vec<String>, String)>,
  /// LLM usage reported by the last completed agent run on this turn.
  /// `input_tokens` is the FULL prompt of the final request — i.e. how
  /// much of the model's context window the conversation filled.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub usage: Option<TurnUsage>,
}

/// Token usage of a completed turn (see [`TurnV2::usage`]).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TurnUsage {
  /// Prompt tokens of the turn's FINAL LLM request.
  pub input_tokens: u64,
  /// Sum of completion tokens across all of the turn's LLM requests.
  pub output_tokens: u64,
}

impl TurnV2 {
  /// Take the given slice of [ConversationMessage]s and convert them into
  /// [TurnV2::responses].
  pub fn assimilate(&mut self, msgs: &[ConversationMessage]) -> Result<(), EngineError> {
    let mut iter = msgs.iter().peekable();
    let mut missing: HashSet<String> = HashSet::new();
    let mut result: Vec<AgentResponse> = vec![];
    while let Some(msg) = iter.next() {
      let ConversationMessage::Assistant { content, tool_calls } = msg else {
        return Err(EngineError::input("expected assistant message"));
      };
      let mut response = AgentResponse {
        content: if content.trim() == "" {
          None
        } else {
          Some(content.clone())
        },
        tool_calls: tool_calls.clone(),
        tool_responses: vec![],
      };

      missing.extend(tool_calls.iter().map(|tc| tc.id.clone()));

      while let Some(msg) = iter.peek() && matches!(msg, ConversationMessage::Tool { .. }) {
        let Some(ConversationMessage::Tool { id, content }) = iter.next() else { unreachable!() };
        missing.remove(id);
        response.tool_responses.push((id.clone(), content.clone()));
      }

      if !missing.is_empty() {
        return Err(EngineError::input(format!("missing tool call responses for {}", itertools::join(missing, ", "))));
      }

      result.push(response);
    }

    self.responses = result;
    Ok(())
  }
}

impl TurnV2 {
  /// Reconstruct the agent-loop conversation from the given retained turns.
  pub fn to_conversation(turns: &[TurnV2]) -> Conversation {
    let mut conv = Conversation::default();
    // We know that we will need AT LEAST turns.len() messages
    conv.messages.reserve(turns.len());
    for turn in turns {
      if let Some(system) = &turn.system {
        conv.push(ConversationMessage::system(system.clone()));
      }
      if let Some(prompt) = &turn.prompt {
        conv.push(ConversationMessage::user(prompt.clone()));
      }

      conv.messages.reserve(turn.responses.len());
      for response in &turn.responses {
        conv.push(ConversationMessage::assistant(
          response.content.as_ref().cloned().unwrap_or_default(),
          response.tool_calls.clone(),
        ));
        conv.messages.reserve(response.tool_responses.len());
        for (id, response) in &response.tool_responses {
          conv.push(ConversationMessage::tool(id.clone(), response.clone()));
        }
      }
    }
    conv
  }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentResponse {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub content: Option<String>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub tool_calls: Vec<ToolCall>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub tool_responses: Vec<(String, String)>,
}

#[cfg(test)]
mod tests {
  use super::*;

  fn assistant(content: &str, tc_id: Option<&str>) -> ConversationMessage {
    ConversationMessage::assistant(
      content.to_string(),
      tc_id.map(|id| vec![ToolCall {
        id: id.to_string(),
        name: "story::note_events".to_string(),
        arguments: "{}".to_string(),
      }]).unwrap_or_default(),
    )
  }

  #[test]
  fn assimilate_assistant_only() {
    let mut turn = TurnV2::default();
    turn.assimilate(&[
      assistant("Hello!", None),
      assistant("World.", None),
    ]).unwrap();
    assert_eq!(turn.responses.len(), 2);
    assert_eq!(turn.responses[0].content.as_deref(), Some("Hello!"));
    assert!(turn.responses[0].tool_calls.is_empty());
  }

  #[test]
  fn assimilate_tool_call_roundtrip() {
    let mut turn = TurnV2::default();
    turn.assimilate(&[
      assistant("", Some("tc1")),
      ConversationMessage::tool("tc1".to_string(), "ok".to_string()),
    ]).unwrap();
    assert_eq!(turn.responses.len(), 1);
    assert!(turn.responses[0].content.is_none());
    assert_eq!(turn.responses[0].tool_calls[0].id, "tc1");
    assert_eq!(turn.responses[0].tool_responses, vec![("tc1".to_string(), "ok".to_string())]);
  }

  #[test]
  fn assimilate_rejects_unanswered_tool_calls() {
    let mut turn = TurnV2::default();
    assert!(turn.assimilate(&[assistant("", Some("tc1"))]).is_err());
  }

  #[test]
  fn assimilate_rejects_non_assistant() {
    let mut turn = TurnV2::default();
    assert!(turn.assimilate(&[ConversationMessage::user("hi".to_string())]).is_err());
  }

  #[test]
  fn turn_serde_roundtrip() {
    let mut turn = TurnV2::default();
    turn.system = Some("Rewrite this.".to_string());
    turn.prompt = Some("look around".to_string());
    turn.edits = vec![(vec!["story".to_string(), "timeline".to_string()], "- [ ] x".to_string())];
    turn.assimilate(&[assistant("You are in a tavern.", None)]).unwrap();

    let json = serde_json::to_string(&turn).unwrap();
    let restored: TurnV2 = serde_json::from_str(&json).unwrap();
    // ToolCall carries no PartialEq, so compare via serialization.
    assert_eq!(serde_json::to_string(&restored).unwrap(), json);

    // Default-able fields may be omitted entirely.
    let bare: TurnV2 = serde_json::from_str("{}").unwrap();
    assert_eq!(serde_json::to_string(&bare).unwrap(), "{}");
  }

  #[test]
  fn to_conversation_rebuilds_messages() {
    let mut turn = TurnV2::default();
    turn.prompt = Some("look around".to_string());
    turn.assimilate(&[assistant("You are in a tavern.", None)]).unwrap();

    let conv = TurnV2::to_conversation(&[turn]);
    assert_eq!(conv.messages.len(), 2);
    assert!(matches!(&conv.messages[0], ConversationMessage::User { content } if content == "look around"));
    assert!(matches!(&conv.messages[1], ConversationMessage::Assistant { content, .. } if content == "You are in a tavern."));
  }
}
