use std::io::{BufWriter, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use tar::Archive;
use tauri::Emitter;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

const USER_AGENT: &str = "ogulniega-manager";

fn http_agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout_read(Duration::from_secs(600))
            .timeout_write(Duration::from_secs(600))
            .build()
    })
}

fn get_mods_dir() -> Result<String, String> {
    if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").map_err(|_| "APPDATA not found".to_string())?;
        Ok(format!("{}/.ogulniega/profile/mods", appdata))
    } else {
        let home = std::env::var("HOME").map_err(|_| "HOME not found".to_string())?;
        Ok(format!("{}/.ogulniega/profile/mods", home))
    }
}

fn get_ogulniega_root() -> Result<PathBuf, String> {
    if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").map_err(|_| "APPDATA not found".to_string())?;
        Ok(PathBuf::from(format!("{}/.ogulniega", appdata)))
    } else {
        let home = std::env::var("HOME").map_err(|_| "HOME not found".to_string())?;
        Ok(PathBuf::from(format!("{}/.ogulniega", home)))
    }
}

fn get_java_dir() -> Result<PathBuf, String> {
    Ok(get_ogulniega_root()?.join("java"))
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

#[tauri::command]
fn write_preinstalled_mod_file(
    game_version: String,
    filename: String,
    data: Vec<u8>,
) -> Result<(), String> {
    let mods_dir = get_mods_dir()?;
    let dest = format!("{}/{}/preinstalled/{}", mods_dir, game_version, filename);
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

#[derive(Serialize)]
struct DuplicateGroup {
    mod_id: String,
    preinstalled: Vec<ModInfo>,
    regular: Vec<ModInfo>,
}

fn extract_mod_id_from_json(json: &serde_json::Value) -> Option<String> {
    if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
        let trimmed = id.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    if let Some(id) = json
        .get("quilt_loader")
        .and_then(|q| q.get("id"))
        .and_then(|v| v.as_str())
    {
        let trimmed = id.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

fn read_fabric_mod_id(jar_path: &Path) -> Option<String> {
    let file = std::fs::File::open(jar_path).ok()?;
    let mut archive = ZipArchive::new(file).ok()?;

    let count = archive.len();
    let candidates: Vec<String> = (0..count)
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .filter(|n| {
            *n == "fabric.mod.json"
                || *n == "quilt.mod.json"
                || n.ends_with("/fabric.mod.json")
                || n.ends_with("/quilt.mod.json")
        })
        .collect();

    for name in candidates {
        let Ok(mut entry) = archive.by_name(&name) else {
            continue;
        };
        let mut content = String::new();
        if entry.read_to_string(&mut content).is_err() {
            continue;
        }
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
            continue;
        };
        if let Some(id) = extract_mod_id_from_json(&json) {
            return Some(id);
        }
    }

    None
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

fn collect_mods_with_ids(dir_path: &Path, sub_path: &str) -> Vec<(String, ModInfo)> {
    let mut result: Vec<(String, ModInfo)> = Vec::new();
    if !dir_path.exists() {
        return result;
    }

    let entries = match std::fs::read_dir(dir_path) {
        Ok(e) => e,
        Err(_) => return result,
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
        let Some(id) = read_fabric_mod_id(&jar_path) else {
            continue;
        };
        if parse_disabled_id(&id).is_some() {
            continue;
        }
        result.push((
            id,
            ModInfo {
                filename,
                sub_path: sub_path.to_string(),
                is_disabled: false,
            },
        ));
    }

    result
}

#[tauri::command]
fn find_duplicate_mods(game_version: String) -> Result<Vec<DuplicateGroup>, String> {
    let mods_dir = get_mods_dir()?;
    let regular_path = PathBuf::from(format!("{}/{}", mods_dir, game_version));
    let preinstalled_path = PathBuf::from(format!("{}/{}/preinstalled", mods_dir, game_version));

    let regular_mods = collect_mods_with_ids(&regular_path, "");
    let preinstalled_mods = collect_mods_with_ids(&preinstalled_path, "preinstalled");

    let mut id_map: std::collections::HashMap<String, (Vec<ModInfo>, Vec<ModInfo>)> =
        std::collections::HashMap::new();

    for (id, info) in regular_mods {
        id_map.entry(id).or_insert_with(|| (Vec::new(), Vec::new())).0.push(info);
    }

    for (id, info) in preinstalled_mods {
        id_map.entry(id).or_insert_with(|| (Vec::new(), Vec::new())).1.push(info);
    }

    let mut duplicates: Vec<DuplicateGroup> = id_map
        .into_iter()
        .filter(|(_, (regular, preinstalled))| regular.len() + preinstalled.len() > 1)
        .map(|(mod_id, (regular, preinstalled))| DuplicateGroup {
            mod_id,
            preinstalled,
            regular,
        })
        .collect();

    duplicates.sort_by(|a, b| a.mod_id.cmp(&b.mod_id));

    Ok(duplicates)
}

#[derive(Serialize, Clone)]
struct PlatformInfo {
    os: String,
    arch: String,
    os_label: String,
    arch_label: String,
}

fn detect_platform() -> PlatformInfo {
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        "unknown"
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "x86") {
        "x32"
    } else if cfg!(target_arch = "arm") {
        "arm"
    } else {
        "unknown"
    };

    let os_label = match os {
        "windows" => "Windows",
        "linux" => "Linux",
        "mac" => "macOS",
        _ => "Unknown OS",
    }
    .to_string();

    let arch_label = match arch {
        "x64" => "x86_64 (x64)",
        "aarch64" => "aarch64 (arm64)",
        "x32" => "x86 (32-bit)",
        "arm" => "arm",
        _ => "Unknown arch",
    }
    .to_string();

    PlatformInfo {
        os: os.to_string(),
        arch: arch.to_string(),
        os_label,
        arch_label,
    }
}

#[derive(Serialize, Clone, Debug)]
struct ParsedJavaName {
    java_name: String,
    major: u32,
    version: String,
    build: u32,
    is_jre: bool,
}

fn parse_java_name(java_name: &str) -> Result<ParsedJavaName, String> {
    let stripped = java_name
        .strip_prefix("jdk-")
        .ok_or_else(|| format!("Invalid java_name (missing 'jdk-' prefix): {}", java_name))?;

    let plus_parts: Vec<&str> = stripped.split('+').collect();
    if plus_parts.len() != 2 {
        return Err(format!(
            "Invalid java_name (expected 'jdk-VERSION+BUILD-KIND'): {}",
            java_name
        ));
    }
    let version = plus_parts[0];
    let build_kind = plus_parts[1];

    let dash_parts: Vec<&str> = build_kind.split('-').collect();
    if dash_parts.is_empty() {
        return Err(format!("Invalid java_name (missing kind): {}", java_name));
    }
    let build: u32 = dash_parts[0]
        .parse()
        .map_err(|_| format!("Invalid build number in {}: {}", java_name, dash_parts[0]))?;
    let kind = if dash_parts.len() > 1 {
        dash_parts[1..].join("-")
    } else {
        "jdk".to_string()
    };
    let is_jre = kind.eq_ignore_ascii_case("jre");

    let major: u32 = version
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| format!("Invalid version in {}: {}", java_name, version))?;

    Ok(ParsedJavaName {
        java_name: java_name.to_string(),
        major,
        version: version.to_string(),
        build,
        is_jre,
    })
}

#[derive(Serialize, Clone, Debug)]
struct JavaProfile {
    java_name: String,
    major: u32,
    version: String,
    build: u32,
    kind: String,
    is_jre: bool,
    usage_count: u32,
}

#[derive(Serialize, Clone, Debug)]
struct LauncherProfilesResponse {
    source: String,
    profiles: Vec<JavaProfile>,
}

fn fetch_launcher_profiles() -> Result<LauncherProfilesResponse, String> {
    const CACHE_TTL: Duration = Duration::from_secs(300);
    static CACHE: OnceLock<std::sync::Mutex<(Instant, LauncherProfilesResponse)>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new((Instant::now() - CACHE_TTL, LauncherProfilesResponse { source: String::new(), profiles: Vec::new() })));
    {
        let guard = cache.lock().unwrap();
        if guard.0.elapsed() < CACHE_TTL {
            return Ok(guard.1.clone());
        }
    }
    let url = "https://ogulniega.com/files/launcher.json";
    let response = http_agent()
        .get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("Launcher JSON request failed: {}", e))?;
    let body = response
        .into_string()
        .map_err(|e| format!("Launcher JSON body read failed: {}", e))?;
    let value: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Launcher JSON parse failed: {}", e))?;

    let mut by_name: std::collections::BTreeMap<String, JavaProfile> =
        std::collections::BTreeMap::new();
    let mut count_by_name: std::collections::HashMap<String, u32> =
        std::collections::HashMap::new();

    if let Some(versions) = value.get("versions").and_then(|v| v.as_array()) {
        for entry in versions {
            let Some(java_name) = entry.get("java_name").and_then(|v| v.as_str()) else {
                continue;
            };
            if java_name.is_empty() {
                continue;
            }
            if let Ok(parsed) = parse_java_name(java_name) {
                if !by_name.contains_key(java_name) {
                    by_name.insert(
                        java_name.to_string(),
                        JavaProfile {
                            java_name: java_name.to_string(),
                            major: parsed.major,
                            version: parsed.version,
                            build: parsed.build,
                            kind: if parsed.is_jre { "jre".to_string() } else { "jdk".to_string() },
                            is_jre: parsed.is_jre,
                            usage_count: 0,
                        },
                    );
                }
                *count_by_name.entry(java_name.to_string()).or_insert(0) += 1;
            }
        }
    }

    let mut profiles: Vec<JavaProfile> = by_name
        .into_values()
        .map(|mut p| {
            p.usage_count = *count_by_name.get(&p.java_name).unwrap_or(&0);
            p
        })
        .collect();
    profiles.sort_by(|a, b| a.major.cmp(&b.major).then(a.java_name.cmp(&b.java_name)));

    if profiles.is_empty() {
        return Err("Launcher JSON did not contain any usable java_name entries".to_string());
    }

    let result = LauncherProfilesResponse {
        source: url.to_string(),
        profiles,
    };
    *cache.lock().unwrap() = (Instant::now(), result.clone());
    Ok(result)
}

#[tauri::command]
async fn get_launcher_profiles() -> Result<LauncherProfilesResponse, String> {
    tauri::async_runtime::spawn_blocking(fetch_launcher_profiles)
        .await
        .map_err(|e| format!("Launcher profiles task panicked: {}", e))?
}

#[derive(Serialize, Clone, Debug)]
struct DistributionInfo {
    id: String,
    label: String,
    description: String,
    kinds: Vec<String>,
    source: String,
}

#[derive(Serialize, Clone, Debug)]
struct ReleaseInfo {
    distribution: String,
    distribution_label: String,
    major: u32,
    kind: String,
    is_jre: bool,
    version: String,
    build: u32,
    java_name: String,
    url: String,
    archive_ext: String,
    size_bytes: u64,
    install_dir: String,
    java_exe: String,
    platform_os: String,
    platform_arch: String,
    source: String,
}

#[derive(Serialize, Clone, Debug, Default, Deserialize)]
struct AdoptiumPackage {
    name: String,
    link: String,
    #[serde(default)]
    size: u64,
}

#[derive(Serialize, Clone, Debug, Default, Deserialize)]
struct AdoptiumBinary {
    os: String,
    architecture: String,
    image_type: String,
    package: AdoptiumPackage,
}

#[derive(Serialize, Clone, Debug, Default, Deserialize)]
struct AdoptiumVersion {
    semver: String,
    major: u32,
    #[serde(default)]
    minor: u32,
    #[serde(default)]
    patch: u32,
    build: u32,
}

#[derive(Serialize, Clone, Debug, Default, Deserialize)]
struct AdoptiumRelease {
    #[serde(default)]
    release_name: String,
    #[serde(default)]
    binaries: Vec<AdoptiumBinary>,
    #[serde(default, rename = "version_data")]
    version: AdoptiumVersion,
}

fn adoptium_os_for(platform: &PlatformInfo) -> Result<&'static str, String> {
    match platform.os.as_str() {
        "windows" => Ok("windows"),
        "linux" => Ok("linux"),
        "mac" => Ok("mac"),
        other => Err(format!("Adoptium has no binary for OS '{}'", other)),
    }
}

fn adoptium_arch_for(platform: &PlatformInfo) -> Result<&'static str, String> {
    match platform.arch.as_str() {
        "x64" => Ok("x64"),
        "aarch64" => Ok("aarch64"),
        "x32" => Ok("x86"),
        "arm" => Ok("arm"),
        other => Err(format!("Adoptium has no binary for arch '{}'", other)),
    }
}

fn http_get_text(url: &str) -> Result<String, String> {
    let response = http_agent()
        .get(url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/json")
        .call()
        .map_err(|e| format!("HTTP GET {} failed: {}", url, e))?;
    response
        .into_string()
        .map_err(|e| format!("HTTP body read failed: {}", e))
}

fn fetch_temurin_latest(
    major: u32,
    kind: &str,
    platform: &PlatformInfo,
) -> Result<(String, u32, String, String, String), String> {
    let os = adoptium_os_for(platform)?;
    let arch = adoptium_arch_for(platform)?;
    let url = format!(
        "https://api.adoptium.net/v3/assets/feature_releases/{major}/ga?architecture={arch}&os={os}&image_type={kind}&page=0&size=1&sort=timestamp&order=DESC",
    );
    let body = http_get_text(&url)?;
    let releases: Vec<AdoptiumRelease> = serde_json::from_str(&body)
        .map_err(|e| format!("Adoptium API parse failed: {}", e))?;
    let release = releases
        .into_iter()
        .next()
        .ok_or_else(|| format!("No Adoptium release for {}/{}/{}", major, kind, arch))?;
    let binary = release
        .binaries
        .into_iter()
        .find(|b| {
            b.os.eq_ignore_ascii_case(&platform.os)
                && b.architecture.eq_ignore_ascii_case(&platform.arch)
                && b.image_type.eq_ignore_ascii_case(kind)
        })
        .ok_or_else(|| {
            format!(
                "No matching Temurin binary for {}-{}-{}",
                platform.os, platform.arch, kind
            )
        })?;
    let ext = if platform.os == "windows" { "zip" } else { "tar.gz" };
    Ok((
        release.version.semver,
        release.version.build,
        binary.package.link,
        ext.to_string(),
        "adoptium-api".to_string(),
    ))
}

fn fetch_liberica_latest(
    major: u32,
    kind: &str,
    platform: &PlatformInfo,
) -> Result<(String, u32, String, String, String), String> {
    let prefix = if kind.eq_ignore_ascii_case("jre") { "jre" } else { "jdk" };
    let (os_token, arch_token, ext) = match (platform.os.as_str(), platform.arch.as_str()) {
        ("windows", "x64") => ("windows", "amd64", "zip"),
        ("windows", "aarch64") => ("windows", "aarch64", "zip"),
        ("linux", "x64") => ("linux", "amd64", "tar.gz"),
        ("linux", "aarch64") => ("linux", "aarch64", "tar.gz"),
        ("mac", "x64") => ("macos", "x64", "tar.gz"),
        ("mac", "aarch64") => ("macos", "aarch64", "tar.gz"),
        _ => return Err(format!("Liberica has no build for {}-{}", platform.os, platform.arch)),
    };
    let api_url = "https://api.github.com/repos/bell-sw/Liberica/releases?per_page=30";
    let body = http_get_text(api_url)?;
    let releases: Vec<serde_json::Value> = serde_json::from_str(&body)
        .map_err(|e| format!("Liberica GitHub API parse failed: {}", e))?;

    for release in &releases {
        let tag = release
            .get("tag_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let tag_major: Option<u32> = tag.split('.').next().and_then(|s| s.parse().ok());
        if tag_major != Some(major) {
            continue;
        }
        let assets = release
            .get("assets")
            .and_then(|a| a.as_array())
            .cloned()
            .unwrap_or_default();
        let target = format!("bellsoft-{prefix}{tag}-{os_token}-{arch_token}.{ext}");
        if let Some(asset) = assets
            .iter()
            .find(|a| a.get("name").and_then(|n| n.as_str()) == Some(target.as_str()))
        {
            let url = asset
                .get("browser_download_url")
                .and_then(|u| u.as_str())
                .unwrap_or("")
                .to_string();
            if url.is_empty() {
                continue;
            }
            let mut parts = tag.split('+');
            let ver = parts.next().unwrap_or(tag).to_string();
            let build_num: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            return Ok((
                ver,
                build_num,
                url,
                ext.to_string(),
                "liberica-github".to_string(),
            ));
        }
    }
    Err(format!(
        "No Liberica release found for Java {} {} on {}-{}",
        major, prefix, os_token, arch_token
    ))
}

fn fetch_zulu_latest(
    major: u32,
    kind: &str,
    platform: &PlatformInfo,
) -> Result<(String, u32, String, String, String), String> {
    let prefix = if kind.eq_ignore_ascii_case("jre") { "jre" } else { "jdk" };
    let (api_os, api_arch) = match (platform.os.as_str(), platform.arch.as_str()) {
        ("windows", "x64") => ("windows", "x86"),
        ("windows", "aarch64") => ("windows", "aarch64"),
        ("linux", "x64") => ("linux", "x64"),
        ("linux", "aarch64") => ("linux", "aarch64"),
        ("mac", "x64") => ("macos", "x64"),
        ("mac", "aarch64") => ("macos", "aarch64"),
        _ => return Err(format!("Zulu has no build for {}-{}", platform.os, platform.arch)),
    };
    let api_url = format!(
        "https://api.azul.com/metadata/v1/zulu/packages?availability_types=ca&release_status=ga&java_version={major}&os={os}&arch={arch}&hw_bitness=64&bundle_type={prefix}&features=latest&latest=true&page_size=50",
        major = major, os = api_os, arch = api_arch, prefix = prefix
    );
    let body = http_get_text(&api_url)?;
    let pkgs: Vec<serde_json::Value> = serde_json::from_str(&body)
        .map_err(|e| format!("Zulu metadata parse failed: {}", e))?;

    let ext_zip = platform.os == "windows";
    let pick = pkgs.into_iter().find(|p| {
        let name = p.get("name").and_then(|n| n.as_str()).unwrap_or("");
        if !name.starts_with("zulu") || !name.contains(prefix) {
            return false;
        }
        if name.contains("-crac-")
            || name.contains("-musl-")
            || name.contains("-fx-")
            || name.contains("-headless-")
        {
            return false;
        }
        if ext_zip {
            name.ends_with(".zip")
        } else {
            name.ends_with(".tar.gz")
        }
    });
    let pkg = pick.ok_or_else(|| {
        format!(
            "No Zulu {} found for major {} on {}-{}",
            prefix,
            major,
            platform.os,
            platform.arch
        )
    })?;
    let url = pkg
        .get("download_url")
        .and_then(|u| u.as_str())
        .ok_or("Zulu: no download_url")?
        .to_string();
    let ext = if ext_zip { "zip" } else { "tar.gz" };
    let java_version = pkg
        .get("java_version")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            let parts: Vec<String> = arr
                .iter()
                .filter_map(|x| x.as_u64().map(|n| n.to_string()))
                .collect();
            if parts.is_empty() {
                None
            } else {
                Some(parts.join("."))
            }
        })
        .unwrap_or_default();
    let build: u32 = pkg
        .get("openjdk_build_number")
        .and_then(|v| v.as_u64())
        .map(|n| n as u32)
        .unwrap_or(0);
    Ok((
        java_version,
        build,
        url,
        ext.to_string(),
        "azul-metadata".to_string(),
    ))
}

fn fetch_corretto_latest(
    major: u32,
    _kind: &str,
    platform: &PlatformInfo,
) -> Result<(String, u32, String, String, String), String> {
    let api_url = format!(
        "https://api.github.com/repos/corretto/corretto-{major}/releases/latest"
    );
    let body = http_get_text(&api_url)?;
    let release: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Corretto API parse failed: {}", e))?;
    let tag = release
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("Corretto major {} not found on GitHub", major))?
        .to_string();
    let (os_token, arch_token, ext, suffix) = match (platform.os.as_str(), platform.arch.as_str()) {
        ("windows", "x64") => ("windows", "x64", "zip", "-jdk"),
        ("windows", "aarch64") => ("windows", "aarch64", "zip", "-jdk"),
        ("linux", "x64") => ("linux", "x64", "tar.gz", ""),
        ("linux", "aarch64") => ("linux", "aarch64", "tar.gz", ""),
        ("mac", "x64") => ("macosx", "x64", "tar.gz", ""),
        ("mac", "aarch64") => ("macosx", "aarch64", "tar.gz", ""),
        _ => return Err(format!("Corretto has no build for {}-{}", platform.os, platform.arch)),
    };
    let asset = format!(
        "amazon-corretto-{tag}-{os_token}-{arch_token}{suffix}.{ext}"
    );
    let url = format!(
        "https://corretto.aws/downloads/resources/{tag}/{asset}"
    );
    let version = tag.clone();
    let build: u32 = 1;
    Ok((version, build, url, ext.to_string(), "corretto-aws".to_string()))
}

fn fetch_microsoft_latest(
    major: u32,
    _kind: &str,
    platform: &PlatformInfo,
) -> Result<(String, u32, String, String, String), String> {
    let version = match major {
        11 => "11.0.31",
        17 => "17.0.19",
        21 => "21.0.11",
        25 => "25.0.2",
        other => {
            return Err(format!(
                "Microsoft OpenJDK has no preset for major {} (known: 11, 17, 21, 25)",
                other
            ))
        }
    };
    let (os_token, arch_token, ext) = match (platform.os.as_str(), platform.arch.as_str()) {
        ("windows", "x64") => ("windows", "x64", "zip"),
        ("windows", "aarch64") => ("windows", "aarch64", "zip"),
        ("linux", "x64") => ("linux", "x64", "tar.gz"),
        ("linux", "aarch64") => ("linux", "aarch64", "tar.gz"),
        ("mac", "x64") => ("macos", "x64", "tar.gz"),
        ("mac", "aarch64") => ("macos", "aarch64", "tar.gz"),
        _ => return Err(format!("Microsoft has no build for {}-{}", platform.os, platform.arch)),
    };
    let asset = format!("microsoft-jdk-{version}-{os_token}-{arch_token}.{ext}");
    let url = format!("https://aka.ms/download-jdk/{asset}");
    let build: u32 = 7;
    Ok((version.to_string(), build, url, ext.to_string(), "microsoft-aka".to_string()))
}

fn build_release_info(
    distribution: &str,
    java_name: &str,
) -> Result<ReleaseInfo, String> {
    static CACHE: OnceLock<std::sync::Mutex<std::collections::HashMap<String, ReleaseInfo>>> =
        OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    let key = format!("{}|{}", distribution, java_name);
    {
        let guard = cache.lock().unwrap();
        if let Some(info) = guard.get(&key) {
            return Ok(info.clone());
        }
    }

    let platform = detect_platform();
    if platform.os == "unknown" || platform.arch == "unknown" {
        return Err(format!(
            "Unsupported platform: os={}, arch={}",
            platform.os, platform.arch
        ));
    }
    let parsed = parse_java_name(java_name)?;
    let major = parsed.major;
    let is_jre = parsed.is_jre;
    let kind_norm = if is_jre { "jre" } else { "jdk" };

    let (version, build, url, archive_ext, source) = match distribution {
        "temurin" | "adoptium" | "eclipse" => {
            fetch_temurin_latest(major, kind_norm, &platform)?
        }
        "liberica" | "bellsoft" => {
            fetch_liberica_latest(major, kind_norm, &platform)?
        }
        "zulu" => {
            fetch_zulu_latest(major, kind_norm, &platform)?
        }
        "corretto" => {
            fetch_corretto_latest(major, kind_norm, &platform)?
        }
        "microsoft" | "ms" | "msopenjdk" => {
            fetch_microsoft_latest(major, kind_norm, &platform)?
        }
        other => {
            return Err(format!(
                "Unknown distribution '{}'. Supported: temurin, zulu, liberica, corretto, microsoft",
                other
            ));
        }
    };

    let java_dir = get_java_dir()?;
    let install_dir = java_dir.join(java_name);
    let java_exe = if cfg!(target_os = "windows") {
        install_dir.join("bin").join("java.exe")
    } else {
        install_dir.join("bin").join("java")
    };

    let distribution_label = match distribution {
        "temurin" | "adoptium" | "eclipse" => "Eclipse Temurin",
        "zulu" => "Azul Zulu",
        "liberica" | "bellsoft" => "BellSoft Liberica",
        "corretto" => "Amazon Corretto",
        "microsoft" | "ms" | "msopenjdk" => "Microsoft OpenJDK",
        _ => distribution,
    }
    .to_string();

    let result = ReleaseInfo {
        distribution: distribution.to_string(),
        distribution_label,
        major,
        kind: kind_norm.to_string(),
        is_jre,
        version: version.to_string(),
        build,
        java_name: java_name.to_string(),
        url,
        archive_ext,
        size_bytes: 0,
        install_dir: install_dir.to_string_lossy().to_string(),
        java_exe: java_exe.to_string_lossy().to_string(),
        platform_os: platform.os.clone(),
        platform_arch: platform.arch.clone(),
        source,
    };
    cache.lock().unwrap().insert(key, result.clone());
    Ok(result)
}

#[tauri::command]
fn get_platform_info() -> PlatformInfo {
    detect_platform()
}

#[tauri::command]
fn get_java_path() -> String {
    match get_java_dir() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => String::new(),
    }
}

#[tauri::command]
fn list_distributions() -> Vec<DistributionInfo> {
    vec![
        DistributionInfo {
            id: "temurin".to_string(),
            label: "Eclipse Temurin".to_string(),
            description: "Adoptium Temurin - the default Adoptium build".to_string(),
            kinds: vec!["jdk".to_string(), "jre".to_string()],
            source: "adoptium-api".to_string(),
        },
        DistributionInfo {
            id: "zulu".to_string(),
            label: "Azul Zulu".to_string(),
            description: "Azul Zulu OpenJDK build".to_string(),
            kinds: vec!["jdk".to_string(), "jre".to_string()],
            source: "static-url".to_string(),
        },
        DistributionInfo {
            id: "liberica".to_string(),
            label: "BellSoft Liberica".to_string(),
            description: "BellSoft Liberica OpenJDK build".to_string(),
            kinds: vec!["jdk".to_string(), "jre".to_string()],
            source: "liberica-api".to_string(),
        },
        DistributionInfo {
            id: "corretto".to_string(),
            label: "Amazon Corretto".to_string(),
            description: "Amazon's no-cost, multiplatform OpenJDK distribution".to_string(),
            kinds: vec!["jdk".to_string(), "jre".to_string()],
            source: "static-url".to_string(),
        },
        DistributionInfo {
            id: "microsoft".to_string(),
            label: "Microsoft OpenJDK".to_string(),
            description: "Microsoft's OpenJDK distribution".to_string(),
            kinds: vec!["jdk".to_string(), "jre".to_string()],
            source: "static-url".to_string(),
        },
    ]
}

#[derive(Serialize)]
struct JavaInstallation {
    java_name: String,
    distribution: String,
    major: u32,
    version: String,
    build: u32,
    is_jre: bool,
    path: String,
    java_exe: String,
    exists: bool,
    java_exe_exists: bool,
    size_bytes: u64,
    parsed: Option<ParsedJavaName>,
    source: String,
}

fn dir_size(path: &Path) -> u64 {
    let mut total: u64 = 0;
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if ft.is_file() {
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
        } else if ft.is_dir() {
            total += dir_size(&entry.path());
        }
    }
    total
}

#[tauri::command]
async fn list_java_installations() -> Result<Vec<JavaInstallation>, String> {
    tauri::async_runtime::spawn_blocking(list_java_installations_blocking)
        .await
        .map_err(|e| format!("List installations task panicked: {}", e))?
}

fn list_java_installations_blocking() -> Result<Vec<JavaInstallation>, String> {
    let java_dir = get_java_dir()?;
    if !java_dir.exists() {
        return Ok(vec![]);
    }

    let mut result: Vec<JavaInstallation> = Vec::new();
    for entry in std::fs::read_dir(&java_dir).map_err(|e| e.to_string())?.flatten() {
        let ft = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if !ft.is_dir() {
            continue;
        }
        let java_name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        let java_exe = if cfg!(target_os = "windows") {
            path.join("bin").join("java.exe")
        } else {
            path.join("bin").join("java")
        };
        let java_exe_exists = java_exe.exists();
        let size = dir_size(&path);
        let parsed = parse_java_name(&java_name).ok();
        let (major, version, build, is_jre) = if let Some(p) = &parsed {
            (p.major, p.version.clone(), p.build, p.is_jre)
        } else {
            (0, String::new(), 0, false)
        };
        result.push(JavaInstallation {
            java_name: java_name.clone(),
            distribution: String::new(),
            major,
            version,
            build,
            is_jre,
            path: path.to_string_lossy().to_string(),
            java_exe: java_exe.to_string_lossy().to_string(),
            exists: true,
            java_exe_exists,
            size_bytes: size,
            parsed,
            source: String::new(),
        });
    }
    result.sort_by(|a, b| a.java_name.cmp(&b.java_name));
    Ok(result)
}

#[tauri::command]
async fn get_java_release(
    distribution: String,
    java_name: String,
) -> Result<ReleaseInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        build_release_info(&distribution, &java_name)
    })
    .await
    .map_err(|e| format!("Release lookup task panicked: {}", e))?
}

fn download_to_file<F>(url: &str, dest: &Path, mut on_progress: F) -> Result<u64, String>
where
    F: FnMut(u64, u64) -> bool,
{
    let response = http_agent()
        .get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("HTTP request failed: {}", e))?;
    let total: u64 = response
        .header("Content-Length")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let mut reader = response.into_reader();
    let file = std::fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::with_capacity(256 * 1024, file);
    let mut buf = vec![0u8; 256 * 1024];
    let mut received: u64 = 0;
    let mut last_emit = Instant::now();
    let _ = on_progress(0, total);
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        writer.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        received += n as u64;
        let should_emit = last_emit.elapsed() >= Duration::from_millis(200)
            || received % (2 * 1024 * 1024) < (256 * 1024);
        if should_emit {
            last_emit = Instant::now();
            if !on_progress(received, total) {
                return Err("Download cancelled".to_string());
            }
        }
    }
    writer.flush().map_err(|e| e.to_string())?;
    let _ = on_progress(received, total);
    Ok(received)
}

fn extract_zip(archive_path: &Path, dest: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let entry_name = entry.name().to_string();
        let outpath = dest.join(&entry_name);

        if entry.is_dir() {
            std::fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut outfile = std::fs::File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut outfile).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            {
                if let Some(mode) = entry.unix_mode() {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode));
                }
            }
        }
    }
    Ok(())
}

fn extract_tar_gz(archive_path: &Path, dest: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive_path).map_err(|e| e.to_string())?;
    let gz = GzDecoder::new(file);
    let mut archive = Archive::new(gz);
    archive.unpack(dest).map_err(|e| format!("tar.gz unpack error: {}", e))?;
    Ok(())
}

fn remove_dir_all_quiet(path: &Path) {
    if path.exists() {
        let _ = std::fs::remove_dir_all(path);
    }
}

fn find_single_top_dir(dir: &Path) -> Option<PathBuf> {
    let entries: Vec<_> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().ok().map(|t| t.is_dir()).unwrap_or(false))
        .collect();
    if entries.len() == 1 {
        Some(entries[0].path())
    } else {
        None
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let ft = entry.file_type()?;
        if ft.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if ft.is_symlink() {
            let target = std::fs::read_link(&src_path)?;
            std::os::unix::fs::symlink(&target, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = std::fs::symlink_metadata(&src_path) {
                    let _ = std::fs::set_permissions(
                        &dst_path,
                        std::fs::Permissions::from_mode(meta.permissions().mode()),
                    );
                }
            }
        }
    }
    Ok(())
}

fn finalize_install_layout(
    temp_extract_dir: &Path,
    install_dir: &Path,
    _java_name: &str,
) -> Result<(), String> {
    if install_dir.exists() {
        remove_dir_all_quiet(install_dir);
    }
    std::fs::create_dir_all(install_dir.parent().ok_or_else(|| "no parent".to_string())?)
        .map_err(|e| e.to_string())?;

    let source: PathBuf = find_single_top_dir(temp_extract_dir)
        .unwrap_or_else(|| temp_extract_dir.to_path_buf());

    copy_dir_recursive(&source, install_dir)
        .map_err(|e| format!("Failed to copy {} -> {}: {}", source.display(), install_dir.display(), e))?;
    Ok(())
}

#[derive(Serialize, Clone)]
struct InstallProgress {
    distribution: String,
    major: u32,
    kind: String,
    java_name: String,
    received: u64,
    total: u64,
    phase: String,
}

#[tauri::command]
async fn install_java(
    app_handle: tauri::AppHandle,
    distribution: String,
    java_name: String,
) -> Result<JavaInstallation, String> {
    tauri::async_runtime::spawn_blocking(move || {
        install_java_blocking(app_handle, distribution, java_name)
    })
    .await
    .map_err(|e| format!("Install task panicked: {}", e))?
}

fn install_java_blocking(
    app_handle: tauri::AppHandle,
    distribution: String,
    java_name: String,
) -> Result<JavaInstallation, String> {
    let info = build_release_info(&distribution, &java_name)?;
    let java_dir = get_java_dir()?;
    std::fs::create_dir_all(&java_dir).map_err(|e| e.to_string())?;

    let install_dir = PathBuf::from(&info.install_dir);
    if install_dir.exists() {
        let java_exe = PathBuf::from(&info.java_exe);
        if java_exe.exists() {
            return Err(format!(
                "Java distribution '{}' is already installed at {}",
                info.java_name, info.install_dir
            ));
        }
        remove_dir_all_quiet(&install_dir);
    }

    let tmp_dir = std::env::temp_dir().join(format!("ogulniega_jdk_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let archive_name = format!(
        "{}.{}",
        info.java_name.replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_"),
        info.archive_ext
    );
    let archive_path = tmp_dir.join(archive_name);
    let extract_stage = tmp_dir.join("extract");

    let _ = app_handle.emit(
        "java-install-progress",
        InstallProgress {
            distribution: info.distribution.clone(),
            major: info.major,
            kind: info.kind.clone(),
            java_name: info.java_name.clone(),
            received: 0,
            total: 0,
            phase: "downloading".to_string(),
        },
    );

    let progress_app = app_handle.clone();
    let progress_info = (
        info.distribution.clone(),
        info.major,
        info.kind.clone(),
        info.java_name.clone(),
    );
    let received = download_to_file(&info.url, &archive_path, |received, total| {
        let _ = progress_app.emit(
            "java-install-progress",
            InstallProgress {
                distribution: progress_info.0.clone(),
                major: progress_info.1,
                kind: progress_info.2.clone(),
                java_name: progress_info.3.clone(),
                received,
                total,
                phase: "downloading".to_string(),
            },
        );
        true
    })
    .map_err(|e| {
        let _ = std::fs::remove_file(&archive_path);
        format!("Failed to download {}: {}", info.url, e)
    })?;

    let _ = app_handle.emit(
        "java-install-progress",
        InstallProgress {
            distribution: info.distribution.clone(),
            major: info.major,
            kind: info.kind.clone(),
            java_name: info.java_name.clone(),
            received,
            total: received,
            phase: "extracting".to_string(),
        },
    );

    if let Err(e) = std::fs::create_dir_all(&extract_stage) {
        let _ = std::fs::remove_file(&archive_path);
        return Err(format!("Failed to create extract dir: {}", e));
    }

    let extract_result = if info.archive_ext == "zip" {
        extract_zip(&archive_path, &extract_stage)
    } else {
        extract_tar_gz(&archive_path, &extract_stage)
    };

    let _ = std::fs::remove_file(&archive_path);

    if let Err(e) = extract_result {
        remove_dir_all_quiet(&extract_stage);
        return Err(e);
    }

    if let Err(e) = finalize_install_layout(&extract_stage, &install_dir, &info.java_name) {
        remove_dir_all_quiet(&extract_stage);
        remove_dir_all_quiet(&install_dir);
        return Err(e);
    }

    let _ = std::fs::remove_dir_all(&extract_stage);

    let java_exe = PathBuf::from(&info.java_exe);
    if !java_exe.exists() {
        if let Some(found) = find_java_executable(&install_dir) {
            return Ok(JavaInstallation {
                java_name: info.java_name.clone(),
                distribution: info.distribution.clone(),
                major: info.major,
                version: info.version.clone(),
                build: info.build,
                is_jre: info.is_jre,
                path: install_dir.to_string_lossy().to_string(),
                java_exe: found.to_string_lossy().to_string(),
                exists: true,
                java_exe_exists: true,
                size_bytes: dir_size(&install_dir),
                parsed: parse_java_name(&info.java_name).ok(),
                source: info.source.clone(),
            });
        }
        remove_dir_all_quiet(&install_dir);
        return Err(format!(
            "Extraction succeeded but java binary not found in {}",
            install_dir.display()
        ));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&java_exe, std::fs::Permissions::from_mode(0o755));
    }

    let _ = app_handle.emit(
        "java-install-progress",
        InstallProgress {
            distribution: info.distribution.clone(),
            major: info.major,
            kind: info.kind.clone(),
            java_name: info.java_name.clone(),
            received,
            total: received,
            phase: "done".to_string(),
        },
    );

    let _ = std::fs::remove_dir_all(&tmp_dir);

    Ok(JavaInstallation {
        java_name: info.java_name.clone(),
        distribution: info.distribution.clone(),
        major: info.major,
        version: info.version.clone(),
        build: info.build,
        is_jre: info.is_jre,
        path: install_dir.to_string_lossy().to_string(),
        java_exe: java_exe.to_string_lossy().to_string(),
        exists: true,
        java_exe_exists: true,
        size_bytes: dir_size(&install_dir),
        parsed: parse_java_name(&info.java_name).ok(),
        source: info.source.clone(),
    })
}

fn find_java_executable(root: &Path) -> Option<PathBuf> {
    let exe_name = if cfg!(target_os = "windows") { "java.exe" } else { "java" };
    let bin = root.join("bin");
    let direct = bin.join(exe_name);
    if direct.exists() {
        return Some(direct);
    }
    if let Ok(entries) = std::fs::read_dir(&bin) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy() == exe_name {
                return Some(entry.path());
            }
        }
    }
    fn walk(dir: &Path, target: &str) -> Option<PathBuf> {
        let entries = std::fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            let ft = entry.file_type().ok()?;
            if ft.is_file() && entry.file_name().to_string_lossy() == target {
                return Some(path);
            }
            if ft.is_dir() {
                if let Some(found) = walk(&path, target) {
                    return Some(found);
                }
            }
        }
        None
    }
    walk(root, exe_name)
}

#[tauri::command]
fn delete_java(java_name: String) -> Result<(), String> {
    let java_dir = get_java_dir()?;
    let path = java_dir.join(&java_name);
    if !path.exists() {
        return Err(format!("JDK not found: {}", path.display()));
    }
    let allowed_root = std::fs::canonicalize(&java_dir).map_err(|e| e.to_string())?;
    let target = std::fs::canonicalize(&path).map_err(|e| e.to_string())?;
    if !target.starts_with(&allowed_root) {
        return Err("Refusing to delete: path escapes java directory".to_string());
    }
    std::fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn parse_java_name_cmd(java_name: String) -> Option<ParsedJavaName> {
    parse_java_name(&java_name).ok()
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
            write_preinstalled_mod_file,
            list_mods,
            disable_mod,
            delete_mod_file,
            find_duplicate_mods,
            get_platform_info,
            get_java_path,
            get_launcher_profiles,
            list_distributions,
            list_java_installations,
            get_java_release,
            install_java,
            delete_java,
            parse_java_name_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
