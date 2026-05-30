use std::path::Path;

fn get_mods_dir() -> Result<String, String> {
    if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").map_err(|_| "APPDATA not found".to_string())?;
        Ok(format!("{}/.ogulniega/profile/mods", appdata))
    } else {
        let home = std::env::var("HOME").map_err(|_| "HOME not found".to_string())?;
        Ok(format!("{}/.ogulniega/profile/mods", home))
    }
}

#[tauri::command]
fn list_game_versions() -> Result<Vec<String>, String> {
    let mods_dir = get_mods_dir()?;
    let path = Path::new(&mods_dir);

    if !path.exists() {
        return Ok(vec![]);
    }

    let mut versions: Vec<String> = std::fs::read_dir(path)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| entry.ok())
        .filter(|e| e.file_type().ok().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    versions.sort();
    Ok(versions)
}

#[tauri::command]
fn list_directory_files(dir_name: String) -> Result<Vec<String>, String> {
    let mods_dir = get_mods_dir()?;
    let dir_path_str = format!("{}/{}", mods_dir, dir_name);
    let path = Path::new(&dir_path_str);

    if !path.exists() {
        return Ok(vec![]);
    }

    let files: Vec<String> = std::fs::read_dir(path)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| entry.ok())
        .filter(|e| e.file_type().ok().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    Ok(files)
}

#[tauri::command]
fn write_mod_file(game_version: String, filename: String, data: Vec<u8>) -> Result<(), String> {
    let mods_dir = get_mods_dir()?;
    let dest = format!("{}/{}/{}", mods_dir, game_version, filename);
    let dest_path = Path::new(&dest);
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(dest_path, &data).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_http::init())
        .invoke_handler(tauri::generate_handler![list_game_versions, list_directory_files, write_mod_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
