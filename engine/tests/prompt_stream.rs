use novelcraft_engine::config::{ModelConfig, ModelPurpose, NovelCraftConfig};
use novelcraft_engine::game::engine::NovelCraftEngine;
use novelcraft_engine::game::session::SessionV2;

/// Serve fixed OpenAI-compatible SSE completions on a random local port,
/// one per incoming connection, in order.
async fn serve_sse(bodies: Vec<String>) -> (String, tokio::task::JoinHandle<()>) {
  use tokio::io::AsyncReadExt;
  use tokio::io::AsyncWriteExt;
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let port = listener.local_addr().unwrap().port();
  let bodies = std::sync::Arc::new(std::sync::Mutex::new(bodies.into_iter()));
  let handle = tokio::spawn(async move {
    loop {
      let Some(body) = bodies.lock().unwrap().next() else { break };
      let (mut sock, _) = listener.accept().await.unwrap();
      // Read the request head (terminated by a blank line).
      let mut buf = vec![0u8; 8192];
      let mut read = 0;
      loop {
        let n = sock.read(&mut buf[read..]).await.unwrap();
        assert!(n > 0, "client closed before sending a request");
        read += n;
        if buf[..read].windows(4).any(|w| w == b"\r\n\r\n") {
          break;
        }
      }
      let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body,
      );
      sock.write_all(response.as_bytes()).await.unwrap();
    }
  });
  (format!("http://127.0.0.1:{port}/v1"), handle)
}

/// Regression: streaming chunks carry explicit nulls ("finish_reason": null
/// on every non-final chunk) which previously failed to parse, so committed
/// turns ended up with an empty agent response.
#[tokio::test]
async fn prompt_commits_streamed_content() {
  let _ = env_logger::try_init();
  let body = concat!(
    "data: {\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":null},\"finish_reason\":null}]}\n\n",
    "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello \"},\"finish_reason\":null}]}\n\n",
    "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"world\"},\"finish_reason\":null}]}\n\n",
    "data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
    "data: [DONE]\n\n",
  );
  let (base_url, server) = serve_sse(vec![body.to_string()]).await;

  let mut config = NovelCraftConfig::default();
  config.models[ModelPurpose::DungeonMaster] = ModelConfig::OpenAi {
    base_url,
    api_key: String::new(),
    model: "test-model".to_string(),
  };

  let mut engine = NovelCraftEngine::new(config);
  let session = engine
    .create_session("Prompt Stream Test".to_string(), "A premise.".to_string())
    .await
    .unwrap();
  engine.set_session(Some(session));

  let (tx, mut rx) = tokio::sync::mpsc::channel(64);
  let drainer = tokio::spawn(async move { while rx.recv().await.is_some() {} });

  engine.prompt("hi".to_string(), tx).await.unwrap();
  drainer.abort();

  // Clean up the persisted session before asserting — tests share the
  // real data directory.
  let id = engine.session().unwrap().id.clone();
  SessionV2::delete(&id).await.unwrap();

  let turn = engine.turn(0).unwrap();
  assert_eq!(turn.prompt.as_deref(), Some("hi"));
  assert_eq!(turn.responses.len(), 1);
  assert_eq!(turn.responses[0].content.as_deref(), Some("Hello world"));
}

/// Regression: `AgentLoop::run` dispatched full namespaced tool names into
/// toolsets whose `handle` matches bare names only, so every tool call
/// failed with "unknown tool" and the gamestate was never updated.
#[tokio::test]
async fn prompt_executes_namespaced_tool_calls() {
  let _ = env_logger::try_init();
  let tool_call = concat!(
    "data: {\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":null,",
    "\"tool_calls\":[{\"index\":0,\"id\":\"t1\",\"type\":\"function\",\"function\":",
    "{\"name\":\"npcs::update_apparel\",\"arguments\":\"{\\\"name\\\":\\\"Mira\\\",\\\"apparel\\\":\\\"red scarf\\\"}\"}}]},",
    "\"finish_reason\":null}]}\n\n",
    "data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n\n",
    "data: [DONE]\n\n",
  );
  let final_body = concat!(
    "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Done.\"},\"finish_reason\":null}]}\n\n",
    "data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
    "data: [DONE]\n\n",
  );
  let (base_url, server) = serve_sse(vec![tool_call.to_string(), final_body.to_string()]).await;

  let mut config = NovelCraftConfig::default();
  config.models[ModelPurpose::DungeonMaster] = ModelConfig::OpenAi {
    base_url,
    api_key: String::new(),
    model: "test".to_string(),
  };

  let mut engine = NovelCraftEngine::new(config);
  let session = engine
    .create_session("Tool Round Trip".to_string(), "A premise.".to_string())
    .await
    .unwrap();
  engine.set_session(Some(session));

  let (tx, mut rx) = tokio::sync::mpsc::channel(64);
  let drainer = tokio::spawn(async move { while rx.recv().await.is_some() {} });

  engine.prompt("give Mira a red scarf".to_string(), tx).await.unwrap();
  drainer.abort();

  let id = engine.session().unwrap().id.clone();
  let npcs_state = engine.module_state("npcs").await.unwrap();
  SessionV2::delete(&id).await.unwrap();

  assert_eq!(npcs_state, serde_json::json!({"Mira": {"apparel": "red scarf"}}));

  let turn = engine.turn(0).unwrap();
  assert_eq!(turn.responses[0].tool_calls[0].name, "npcs::update_apparel");
  assert_eq!(
    turn.responses[0].tool_responses[0].1,
    "Apparel of Mira successfully updated."
  );
  assert_eq!(turn.responses[1].content.as_deref(), Some("Done."));
}
