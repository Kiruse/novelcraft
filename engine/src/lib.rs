mod commands;
pub mod config;
pub mod error;
pub mod game;
mod infer;
mod util;

use commands as cmds;

use crate::game::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  env_logger::init();
  let builder = tauri_specta::Builder::<tauri::Wry>::new()
    .commands(tauri_specta::collect_commands![
      cmds::fs::datapath,
      cmds::game::game_fork,
      cmds::game::game_page,
      cmds::game::game_prompt,
      cmds::game::game_sessions,
      cmds::llm::prompt,
      cmds::llm::list_models,
      cmds::llm::save_models,
      cmds::llm::ping_hosts,
      cmds::llm::ping_host,
      cmds::session::session_list,
      cmds::session::session_create,
      cmds::session::session_delete,
      cmds::session::session_load,
      cmds::session::session_save_meta,
      cmds::session::session_upsert_page,
      cmds::session::session_truncate_pages,
      cmds::session::session_get_head_snapshot,
      cmds::session::session_save_head_snapshot,
      cmds::session::session_delete_head_snapshot,
      cmds::session::session_find_snapshot_before,
      cmds::session::session_save_checkpoint,
      cmds::session::session_delete_checkpoints_from,
      cmds::profile::profile_list,
      cmds::profile::profile_create,
      cmds::profile::profile_update,
      cmds::profile::profile_delete,
      cmds::profile::profile_set_active,
      cmds::story::story_get,
      cmds::story::story_save,
      cmds::lore::lore_query,
    ]);

  #[cfg(debug_assertions)]
  builder
    .export(
      specta_typescript::Typescript::default(),
      "../gui/src/bindings.ts",
    )
    .expect("Failed to export typescript bindings");

  tauri::Builder::default()
    .manage(AppState::default())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_fs::init())
    .plugin(tauri_plugin_store::Builder::default().build())
    .invoke_handler(builder.invoke_handler())
    .setup(move |app| {
      let app_handle = app.handle().clone();
      tauri::async_runtime::spawn(async move {
        AppState::init(&app_handle).await.expect("Failed to initialize app state");
      });
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
