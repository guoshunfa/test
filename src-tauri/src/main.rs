mod commands; // 导入 commands 模块

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![commands::start_editing]) // 注意添加模块名前缀
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}