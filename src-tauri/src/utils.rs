use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct GameConfig {
    pub name: String,
    pub display_name: String,
    pub path: String,
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub steam_app_id: u32,
}

pub struct Games;

impl Games {
    pub fn get_config(game_key: &str) -> Option<GameConfig> {
        match game_key {
            "hk" => Some(GameConfig {
                name: "Hollow Knight".to_string(),
                display_name: "Hollow Knight".to_string(),
                path: "Hollow Knight".to_string(),
                steam_app_id: 367520,
            }),
            "silksong" => Some(GameConfig {
                name: "Silksong".to_string(),
                display_name: "Silksong".to_string(),
                path: "Hollow Knight Silksong".to_string(),
                steam_app_id: 1030300,
            }),
            _ => None,
        }
    }
}

pub fn get_game_paths(game_key: Option<&str>) -> Vec<(String, PathBuf)> {
    let mut all_paths = Vec::new();
    let games_to_check: Vec<&str> = match game_key {
        Some(key) => vec![key],
        None => vec!["hk", "silksong"],
    };

    for game in games_to_check {
        if let Some(game_config) = Games::get_config(game) {
            let mut seen = std::collections::HashSet::new();
            for path in platform_save_paths(&game_config) {
                if seen.insert(dedup_key(&path)) {
                    all_paths.push((game.to_string(), path));
                }
            }
        }
    }

    all_paths
}

/// Key used to detect duplicate search paths. Resolves symlinks where possible
/// (e.g. `~/.steam/steam` -> `~/.local/share/Steam`); Windows paths are
/// case-insensitive, so they are compared lowercased.
fn dedup_key(path: &Path) -> String {
    let resolved = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let key = resolved.to_string_lossy().to_string();
    if cfg!(target_os = "windows") {
        key.to_lowercase()
    } else {
        key
    }
}

#[cfg(target_os = "windows")]
fn platform_save_paths(game_config: &GameConfig) -> Vec<PathBuf> {
    let mut game_paths = Vec::new();

    if let Some(home_dir) = dirs::home_dir() {
        game_paths.push(
            home_dir
                .join("AppData")
                .join("LocalLow")
                .join("Team Cherry")
                .join(&game_config.path),
        );
    }

    if let Ok(username) = std::env::var("USERNAME") {
        game_paths.push(
            PathBuf::from("C:")
                .join("Users")
                .join(username)
                .join("AppData")
                .join("LocalLow")
                .join("Team Cherry")
                .join(&game_config.path),
        );
    }

    game_paths
}

#[cfg(target_os = "linux")]
fn platform_save_paths(game_config: &GameConfig) -> Vec<PathBuf> {
    let Some(home_dir) = dirs::home_dir() else {
        return Vec::new();
    };
    let config_dir = dirs::config_dir().unwrap_or_else(|| home_dir.join(".config"));
    linux_save_paths(&home_dir, &config_dir, game_config)
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn platform_save_paths(_game_config: &GameConfig) -> Vec<PathBuf> {
    Vec::new()
}

/// Flatpak ID of the Steam client on Flathub.
#[cfg(any(target_os = "linux", test))]
const STEAM_FLATPAK_ID: &str = "com.valvesoftware.Steam";

/// All places a Linux install may keep Hollow Knight / Silksong saves:
/// the native Linux build (Unity stores it under `~/.config/unity3d`) and the
/// Windows build running through Proton (inside the game's Wine prefix in
/// whichever Steam library the game is installed to).
#[cfg(any(target_os = "linux", test))]
fn linux_save_paths(home_dir: &Path, config_dir: &Path, game_config: &GameConfig) -> Vec<PathBuf> {
    let mut game_paths = Vec::new();
    let flatpak_home = home_dir.join(".var").join("app").join(STEAM_FLATPAK_ID);

    // native Linux build
    for unity_config in [config_dir.to_path_buf(), flatpak_home.join(".config")] {
        game_paths.push(
            unity_config
                .join("unity3d")
                .join("Team Cherry")
                .join(&game_config.path),
        );
    }

    // Windows build via Proton
    let steam_roots = [
        home_dir.join(".local").join("share").join("Steam"),
        home_dir.join(".steam").join("steam"),
        home_dir.join(".steam").join("root"),
        flatpak_home.join("data").join("Steam"),
        flatpak_home.join(".local").join("share").join("Steam"),
    ];

    let mut libraries: Vec<PathBuf> = Vec::new();
    for root in &steam_roots {
        if !root.exists() {
            continue;
        }
        libraries.push(root.clone());
        let vdf = root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(content) = std::fs::read_to_string(&vdf) {
            libraries.extend(parse_library_folders(&content));
        }
    }

    for library in libraries {
        game_paths.push(
            library
                .join("steamapps")
                .join("compatdata")
                .join(game_config.steam_app_id.to_string())
                .join("pfx")
                .join("drive_c")
                .join("users")
                .join("steamuser")
                .join("AppData")
                .join("LocalLow")
                .join("Team Cherry")
                .join(&game_config.path),
        );
    }

    game_paths
}

/// Extracts the library paths from Steam's `libraryfolders.vdf`, e.g.
/// `"path"    "/mnt/games/SteamLibrary"`.
#[cfg(any(target_os = "linux", test))]
fn parse_library_folders(content: &str) -> Vec<PathBuf> {
    let re = regex::Regex::new(r#""path"\s+"((?:[^"\\]|\\.)*)""#).expect("valid regex");
    re.captures_iter(content)
        .map(|c| PathBuf::from(c[1].replace("\\\\", "\\")))
        .collect()
}

pub fn is_jksv_format(file_path: &str) -> bool {
    let path = std::path::Path::new(file_path);

    match path.extension().and_then(|ext| ext.to_str()) {
        // zip file (legacy support)
        Some(ext) if ext.eq_ignore_ascii_case("zip") => true,
        // JKSV folder backup: an existing directory, or a not-yet-created
        // path whose file name has no extension. Only the final component is
        // checked so that dots in parent folders (e.g. `~/.local`) don't matter.
        Some(_) => path.is_dir(),
        None => path.is_dir() || !path.exists(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_steam_library_folders() {
        let vdf = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"/home/user/.local/share/Steam"
		"label"		""
		"apps"
		{
			"367520"		"9437184000"
		}
	}
	"1"
	{
		"path"		"/mnt/Games Drive/SteamLibrary"
	}
}
"#;
        assert_eq!(
            parse_library_folders(vdf),
            vec![
                PathBuf::from("/home/user/.local/share/Steam"),
                PathBuf::from("/mnt/Games Drive/SteamLibrary"),
            ]
        );
    }

    #[test]
    fn linux_paths_cover_native_and_proton() {
        let tmp = std::env::temp_dir().join(format!("hollow-sync-test-{}", std::process::id()));
        let home = tmp.join("home");
        let steam = home.join(".local/share/Steam");
        let extra_lib = tmp.join("SteamLibrary");
        std::fs::create_dir_all(steam.join("steamapps")).unwrap();
        std::fs::write(
            steam.join("steamapps/libraryfolders.vdf"),
            format!("\"0\" {{ \"path\" \"{}\" }}", extra_lib.display()),
        )
        .unwrap();

        let game = Games::get_config("silksong").unwrap();
        let paths = linux_save_paths(&home, &home.join(".config"), &game);

        assert!(paths.contains(&home.join(".config/unity3d/Team Cherry/Hollow Knight Silksong")));
        assert!(paths.contains(&home.join(
            ".var/app/com.valvesoftware.Steam/.config/unity3d/Team Cherry/Hollow Knight Silksong"
        )));
        let proton_suffix = "steamapps/compatdata/1030300/pfx/drive_c/users/steamuser/AppData/LocalLow/Team Cherry/Hollow Knight Silksong";
        assert!(paths.contains(&steam.join(proton_suffix)));
        assert!(paths.contains(&extra_lib.join(proton_suffix)));

        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn jksv_format_ignores_dots_in_parent_dirs() {
        assert!(is_jksv_format(
            "/home/user/.local/share/HollowSync/saves/hk-slot1"
        ));
        assert!(is_jksv_format(
            "/home/user/.local/share/HollowSync/backup.zip"
        ));
        assert!(!is_jksv_format(
            "/home/user/.local/share/HollowSync/saves/user1.dat"
        ));
    }
}
