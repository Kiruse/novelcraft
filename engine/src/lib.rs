mod commands;

use commands::{fs as fs_cmds, llm as llm_cmds};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_sql::Builder::default().build())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_fs::init())
    .invoke_handler(tauri::generate_handler![
      llm_cmds::prompt,
      llm_cmds::list_models,
      llm_cmds::save_models,
      fs_cmds::export_session,
      fs_cmds::import_session,
      fs_cmds::pick_file,
      fs_cmds::pick_folder,
    ])
    .setup(|app| {
      let app_handle = app.handle().clone();
      tauri::async_runtime::spawn(async move {
        llm_cmds::init_models(&app_handle).await;
      });
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
