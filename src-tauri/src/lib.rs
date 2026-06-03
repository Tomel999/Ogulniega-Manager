use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

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

#[derive(Serialize)]
struct ModInfo {
    filename: String,
    sub_path: String,
    is_disabled: bool,
}

fn read_fabric_mod_id(jar_path: &Path) -> Option<String> {
    let file = std::fs::File::open(jar_path).ok()?;
    let mut archive = ZipArchive::new(file).ok()?;
    let mut entry = archive.by_name("fabric.mod.json").ok()?;
    let mut content = String::new();
    entry.read_to_string(&mut content).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    json.get("id")?.as_str().map(|s| s.to_string())
}

fn parse_disabled_id(id: &str) -> Option<u32> {
    id.strip_prefix("disabledmod")
        .and_then(|rest| rest.parse::<u32>().ok())
}

fn version_dirs(mods_dir: &str, game_version: &str) -> Vec<(String, PathBuf)> {
    vec![
        (
            String::new(),
            PathBuf::from(format!("{}/{}", mods_dir, game_version)),
        ),
        (
            "preinstalled".to_string(),
            PathBuf::from(format!("{}/{}/preinstalled", mods_dir, game_version)),
        ),
    ]
}

#[tauri::command]
fn list_mods(game_version: String) -> Result<Vec<ModInfo>, String> {
    let mods_dir = get_mods_dir()?;
    let mut result: Vec<ModInfo> = Vec::new();

    for (sub_path, dir_path) in version_dirs(&mods_dir, &game_version) {
        if !dir_path.exists() {
            continue;
        }

        let entries = match std::fs::read_dir(&dir_path) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.filter_map(|e| e.ok()) {
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            let filename = entry.file_name().to_string_lossy().to_string();
            if !filename.to_lowercase().ends_with(".jar") {
                continue;
            }
            let jar_path = entry.path();
            let id = read_fabric_mod_id(&jar_path);
            let is_disabled = id.as_deref().and_then(parse_disabled_id).is_some();
            result.push(ModInfo {
                filename,
                sub_path: sub_path.clone(),
                is_disabled,
            });
        }
    }

    result.sort_by(|a, b| {
        a.sub_path
            .cmp(&b.sub_path)
            .then_with(|| a.filename.to_lowercase().cmp(&b.filename.to_lowercase()))
    });

    Ok(result)
}

fn next_disabled_index(mods_dir: &str, game_version: &str) -> u32 {
    let mut max_n: u32 = 0;
    for (_, dir_path) in version_dirs(mods_dir, game_version) {
        if !dir_path.exists() {
            continue;
        }
        let entries = match std::fs::read_dir(&dir_path) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.filter_map(|e| e.ok()) {
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.to_lowercase().ends_with(".jar") {
                continue;
            }
            if let Some(id) = read_fabric_mod_id(&entry.path()) {
                if let Some(n) = parse_disabled_id(&id) {
                    if n > max_n {
                        max_n = n;
                    }
                }
            }
        }
    }
    max_n + 1
}

fn build_stub_jar(disabled_id: &str) -> Result<Vec<u8>, String> {
    let fabric_json = serde_json::json!({
        "schemaVersion": 1,
        "id": disabled_id,
        "version": "1.0.0",
        "name": "mod is disabled, check files",
        "description": "Placeholder",
        "environment": "*",
        "entrypoints": {}
    });
    let json_content = serde_json::to_string(&fabric_json).map_err(|e| e.to_string())?;

    let buf: Vec<u8> = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    zip.start_file("fabric.mod.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(json_content.as_bytes())
        .map_err(|e| e.to_string())?;
    let cursor = zip.finish().map_err(|e| e.to_string())?;
    Ok(cursor.into_inner())
}

#[tauri::command]
fn disable_mod(
    game_version: String,
    sub_path: String,
    filename: String,
) -> Result<String, String> {
    let mods_dir = get_mods_dir()?;
    let dir_path_str = if sub_path.is_empty() {
        format!("{}/{}", mods_dir, game_version)
    } else {
        format!("{}/{}/{}", mods_dir, game_version, sub_path)
    };
    let dir_path = Path::new(&dir_path_str);
    let file_path = dir_path.join(&filename);

    if !file_path.exists() {
        return Err(format!("File does not exist: {}", file_path.display()));
    }

    if let Some(id) = read_fabric_mod_id(&file_path) {
        if parse_disabled_id(&id).is_some() {
            return Ok(id);
        }
    }

    let n = next_disabled_index(&mods_dir, &game_version);
    let new_id = format!("disabledmod{}", n);

    let jar_bytes = build_stub_jar(&new_id)?;

    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&file_path, &jar_bytes).map_err(|e| e.to_string())?;

    Ok(new_id)
}

#[tauri::command]
fn delete_mod_file(
    game_version: String,
    sub_path: String,
    filename: String,
) -> Result<(), String> {
    let mods_dir = get_mods_dir()?;
    let dir_path_str = if sub_path.is_empty() {
        format!("{}/{}", mods_dir, game_version)
    } else {
        format!("{}/{}/{}", mods_dir, game_version, sub_path)
    };
    let file_path = Path::new(&dir_path_str).join(&filename);

    if !file_path.exists() {
        return Err(format!("File does not exist: {}", file_path.display()));
    }

    std::fs::remove_file(&file_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_http::init())
        .invoke_handler(tauri::generate_handler![
            list_game_versions,
            list_directory_files,
            write_mod_file,
            list_mods,
            disable_mod,
            delete_mod_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
