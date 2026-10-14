use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::simulation::game_mode::MatchStats;

pub const MAX_SAVED_RESULTS: usize = 50;
const DISPLAY_RESOLUTIONS: [(u32, u32); 4] = [(960, 540), (1280, 720), (1600, 900), (1920, 1080)];
const RENDER_SCALES: [f32; 4] = [0.5, 0.67, 0.75, 1.0];
const FPS_LIMITS: [u16; 7] = [30, 60, 90, 120, 144, 240, 0];
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingKey {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Space,
    LeftShift,
    RightShift,
    Escape,
    Digit1,
    Digit2,
    Digit3,
}

impl BindingKey {
    pub fn label(self) -> &'static str {
        match self {
            Self::Space => "SPACE",
            Self::LeftShift => "L-SHIFT",
            Self::RightShift => "R-SHIFT",
            Self::Escape => "ESC",
            Self::Digit1 => "1",
            Self::Digit2 => "2",
            Self::Digit3 => "3",
            key => match key {
                Self::A => "A",
                Self::B => "B",
                Self::C => "C",
                Self::D => "D",
                Self::E => "E",
                Self::F => "F",
                Self::G => "G",
                Self::H => "H",
                Self::I => "I",
                Self::J => "J",
                Self::K => "K",
                Self::L => "L",
                Self::M => "M",
                Self::N => "N",
                Self::O => "O",
                Self::P => "P",
                Self::Q => "Q",
                Self::R => "R",
                Self::S => "S",
                Self::T => "T",
                Self::U => "U",
                Self::V => "V",
                Self::W => "W",
                Self::X => "X",
                Self::Y => "Y",
                Self::Z => "Z",
                _ => unreachable!(),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingAction {
    Forward,
    Backward,
    Left,
    Right,
    Dash,
    Jump,
    Reload,
    Pistol,
    AssaultRifle,
    Sniper,
    Pause,
}

impl BindingAction {
    pub const ALL: [Self; 11] = [
        Self::Forward,
        Self::Backward,
        Self::Left,
        Self::Right,
        Self::Dash,
        Self::Jump,
        Self::Reload,
        Self::Pistol,
        Self::AssaultRifle,
        Self::Sniper,
        Self::Pause,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Forward => "MOVE FORWARD",
            Self::Backward => "MOVE BACK",
            Self::Left => "MOVE LEFT",
            Self::Right => "MOVE RIGHT",
            Self::Dash => "DASH",
            Self::Jump => "JUMP",
            Self::Reload => "RELOAD",
            Self::Pistol => "PISTOL",
            Self::AssaultRifle => "ASSAULT RIFLE",
            Self::Sniper => "SNIPER",
            Self::Pause => "PAUSE",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct KeyBindings {
    pub forward: BindingKey,
    pub backward: BindingKey,
    pub left: BindingKey,
    pub right: BindingKey,
    pub dash: BindingKey,
    pub jump: BindingKey,
    pub reload: BindingKey,
    pub pistol: BindingKey,
    pub assault_rifle: BindingKey,
    pub sniper: BindingKey,
    pub pause: BindingKey,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            forward: BindingKey::W,
            backward: BindingKey::S,
            left: BindingKey::A,
            right: BindingKey::D,
            dash: BindingKey::LeftShift,
            jump: BindingKey::Space,
            reload: BindingKey::R,
            pistol: BindingKey::Digit1,
            assault_rifle: BindingKey::Digit2,
            sniper: BindingKey::Digit3,
            pause: BindingKey::Escape,
        }
    }
}

impl KeyBindings {
    pub fn get(self, action: BindingAction) -> BindingKey {
        match action {
            BindingAction::Forward => self.forward,
            BindingAction::Backward => self.backward,
            BindingAction::Left => self.left,
            BindingAction::Right => self.right,
            BindingAction::Dash => self.dash,
            BindingAction::Jump => self.jump,
            BindingAction::Reload => self.reload,
            BindingAction::Pistol => self.pistol,
            BindingAction::AssaultRifle => self.assault_rifle,
            BindingAction::Sniper => self.sniper,
            BindingAction::Pause => self.pause,
        }
    }

    pub fn set(&mut self, action: BindingAction, key: BindingKey) -> bool {
        if BindingAction::ALL
            .iter()
            .any(|other| *other != action && self.get(*other) == key)
        {
            return false;
        }
        match action {
            BindingAction::Forward => self.forward = key,
            BindingAction::Backward => self.backward = key,
            BindingAction::Left => self.left = key,
            BindingAction::Right => self.right = key,
            BindingAction::Dash => self.dash = key,
            BindingAction::Jump => self.jump = key,
            BindingAction::Reload => self.reload = key,
            BindingAction::Pistol => self.pistol = key,
            BindingAction::AssaultRifle => self.assault_rifle = key,
            BindingAction::Sniper => self.sniper = key,
            BindingAction::Pause => self.pause = key,
        }
        true
    }

    fn are_unique(self) -> bool {
        BindingAction::ALL
            .into_iter()
            .enumerate()
            .all(|(index, action)| {
                BindingAction::ALL[index + 1..]
                    .iter()
                    .all(|other| self.get(action) != self.get(*other))
            })
    }
}

fn write_atomically(path: &Path, temp_path: &Path, data: &[u8]) -> io::Result<()> {
    let mut temp_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp_path)?;
    let result = (|| {
        temp_file.write_all(data)?;
        temp_file.sync_all()?;
        drop(temp_file);
        replace_file(temp_path, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp_path);
    }
    result
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING};

    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // SAFETY: Both UTF-16 paths are NUL-terminated and remain alive for the call.
    let moved = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING,
        )
    };
    if moved == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn write_file_atomically(path: &Path, data: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "destination has no file name")
    })?;
    for _ in 0..16 {
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp_path = parent.join(format!(
            ".{}.{}.{}.tmp",
            file_name.to_string_lossy(),
            std::process::id(),
            sequence
        ));
        match write_atomically(path, &temp_path, data) {
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            result => return result,
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not reserve a temporary file for atomic write",
    ))
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResultRecord {
    pub recorded_at_unix_seconds: u64,
    pub stats: MatchStats,
}

pub fn save_result(stats: MatchStats) -> Result<(), String> {
    let path = GameSettings::path()
        .map(|path| path.with_file_name("results.json"))
        .ok_or_else(|| "結果保存先を特定できません".to_owned())?;
    let recorded_at_unix_seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    save_result_to(
        &path,
        ResultRecord {
            recorded_at_unix_seconds,
            stats,
        },
    )
}

pub fn load_results_from(path: &std::path::Path) -> Result<Vec<ResultRecord>, String> {
    match fs::read_to_string(path) {
        Ok(json) => Ok(serde_json::from_str(&json).unwrap_or_default()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error.to_string()),
    }
}

fn corrupt_results_backup_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("results.json"))
        .to_string_lossy();
    loop {
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let backup = path.with_file_name(format!(
            "{file_name}.corrupt.{}.{}",
            std::process::id(),
            sequence
        ));
        if !backup.exists() {
            return backup;
        }
    }
}

pub fn save_result_to(path: &std::path::Path, record: ResultRecord) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "結果保存先が不正です".to_owned())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let mut records = match fs::read_to_string(path) {
        Ok(json) => match serde_json::from_str(&json) {
            Ok(records) => records,
            Err(_) => {
                let backup = corrupt_results_backup_path(path);
                fs::rename(path, backup).map_err(|error| error.to_string())?;
                Vec::new()
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(error.to_string()),
    };
    records.push(record);
    if records.len() > MAX_SAVED_RESULTS {
        records.drain(..records.len() - MAX_SAVED_RESULTS);
    }
    let data = serde_json::to_vec_pretty(&records).map_err(|error| error.to_string())?;
    write_file_atomically(path, &data).map_err(|error| error.to_string())
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct GameSettings {
    pub mouse_sensitivity: f32,
    pub ads_sensitivity: f32,
    pub invert_y: bool,
    pub fov: f32,
    pub fullscreen: bool,
    pub vsync: bool,
    pub display_resolution: u8,
    pub render_scale_index: u8,
    pub fps_limit_index: u8,
    pub key_bindings: KeyBindings,
    pub infinite_ammo: bool,
    pub volume: f32,
    pub hit_marker_strength: f32,
    pub bot_count: u32,
    /// 0 = Static, 1 = Strafe, 2 = Chase, 3 = Mixed.
    pub bot_behavior: u8,
    pub bot_hp: u32,
    pub bot_speed: f32,
    pub bot_respawn_seconds: f32,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 1.0,
            ads_sensitivity: 0.65,
            invert_y: false,
            fov: 70.0,
            fullscreen: false,
            vsync: true,
            display_resolution: 0,
            render_scale_index: 1,
            fps_limit_index: 6,
            key_bindings: KeyBindings::default(),
            infinite_ammo: false,
            volume: 0.7,
            hit_marker_strength: 1.0,
            bot_count: 5,
            bot_behavior: 3,
            bot_hp: 100,
            bot_speed: 1.0,
            bot_respawn_seconds: 2.0,
        }
    }
}

impl GameSettings {
    pub fn look_multiplier(&self, ads_progress: f32) -> f32 {
        self.mouse_sensitivity * (1.0 + (self.ads_sensitivity - 1.0) * ads_progress.clamp(0.0, 1.0))
    }

    pub fn look_y_sign(&self) -> f32 {
        if self.invert_y {
            -1.0
        } else {
            1.0
        }
    }

    pub fn resolution_size(&self) -> (u32, u32) {
        DISPLAY_RESOLUTIONS[usize::from(self.display_resolution.min(3))]
    }

    pub fn render_scale(&self) -> f32 {
        RENDER_SCALES[usize::from(self.render_scale_index.min(3))]
    }

    pub fn fps_limit(&self) -> Option<u16> {
        match FPS_LIMITS[usize::from(self.fps_limit_index.min(6))] {
            0 => None,
            fps => Some(fps),
        }
    }

    pub fn toggle_setting(&mut self, setting: usize) {
        match setting {
            0 => self.infinite_ammo = !self.infinite_ammo,
            1 => self.fullscreen = !self.fullscreen,
            2 => self.invert_y = !self.invert_y,
            3 => self.vsync = !self.vsync,
            _ => {}
        }
    }

    pub fn adjust_sensitivity(&mut self, setting: usize, direction: f32) {
        match setting {
            0 => {
                self.mouse_sensitivity = (self.mouse_sensitivity + direction * 0.1).clamp(0.1, 5.0)
            }
            1 => self.ads_sensitivity = (self.ads_sensitivity + direction * 0.05).clamp(0.1, 1.0),
            _ => {}
        }
    }

    pub fn adjust_setting(&mut self, setting: usize, direction: f32) {
        if setting < 2 {
            self.adjust_sensitivity(setting, direction);
        } else if setting == 2 {
            self.fov = (self.fov + direction).clamp(60.0, 120.0);
        } else if setting == 3 {
            self.bot_count = (self.bot_count as f32 + direction.signum()).clamp(1.0, 20.0) as u32;
        } else if setting == 4 && direction != 0.0 {
            self.bot_behavior =
                match (self.bot_behavior as i8 + direction.signum() as i8).rem_euclid(4) {
                    0 => 0,
                    1 => 1,
                    2 => 2,
                    _ => 3,
                };
        } else if setting == 5 {
            self.volume = (self.volume + direction * 0.05).clamp(0.0, 1.0);
        } else if setting == 6 {
            self.bot_hp =
                (self.bot_hp as f32 + direction.signum() * 10.0).clamp(50.0, 200.0) as u32;
        } else if setting == 7 {
            self.bot_speed = (self.bot_speed + direction * 0.1).clamp(0.5, 2.0);
        } else if setting == 8 {
            self.bot_respawn_seconds =
                (self.bot_respawn_seconds + direction * 0.25).clamp(0.5, 10.0);
        } else if setting == 9 && direction != 0.0 {
            self.display_resolution = (i16::from(self.display_resolution)
                + direction.signum() as i16)
                .rem_euclid(4) as u8;
        } else if setting == 10 && direction != 0.0 {
            self.render_scale_index = (i16::from(self.render_scale_index)
                + direction.signum() as i16)
                .rem_euclid(4) as u8;
        } else if setting == 11 && direction != 0.0 {
            self.fps_limit_index =
                (i16::from(self.fps_limit_index) + direction.signum() as i16).rem_euclid(7) as u8;
        } else if setting == 12 {
            self.hit_marker_strength = (self.hit_marker_strength + direction * 0.1).clamp(0.0, 1.0);
        }
    }

    pub fn from_json(json: &str) -> Self {
        let Ok(settings) = serde_json::from_str::<Self>(json) else {
            return Self::default();
        };
        if !(0.1..=5.0).contains(&settings.mouse_sensitivity)
            || !(0.1..=1.0).contains(&settings.ads_sensitivity)
            || !(60.0..=120.0).contains(&settings.fov)
            || !(0.0..=1.0).contains(&settings.volume)
            || !(0.0..=1.0).contains(&settings.hit_marker_strength)
            || !(1..=20).contains(&settings.bot_count)
            || settings.bot_behavior > 3
            || !(50..=200).contains(&settings.bot_hp)
            || !(0.5..=2.0).contains(&settings.bot_speed)
            || !(0.5..=10.0).contains(&settings.bot_respawn_seconds)
            || settings.display_resolution > 3
            || settings.render_scale_index > 3
            || settings.fps_limit_index > 6
            || !settings.key_bindings.are_unique()
        {
            return Self::default();
        }
        settings
    }

    pub fn load() -> Self {
        let Some(path) = Self::path() else {
            return Self::default();
        };
        Self::load_from(&path)
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path().ok_or_else(|| "設定保存先を特定できません".to_owned())?;
        Self::save_to(&path, self)
    }

    fn load_from(path: &std::path::Path) -> Self {
        fs::read_to_string(path)
            .map(|json| Self::from_json(&json))
            .unwrap_or_default()
    }

    fn save_to(path: &std::path::Path, settings: &Self) -> Result<(), String> {
        let parent = path
            .parent()
            .ok_or_else(|| "設定保存先が不正です".to_owned())?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let data = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
        write_file_atomically(path, &data).map_err(|e| e.to_string())
    }

    fn path() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        let root = std::env::var_os("APPDATA").map(PathBuf::from)?;
        #[cfg(target_os = "macos")]
        let root = std::env::var_os("HOME")
            .map(PathBuf::from)?
            .join("Library/Application Support");
        #[cfg(all(unix, not(target_os = "macos")))]
        let root = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
        Some(root.join("liteweight-fps").join("settings.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::GameSettings;
    use std::fs;

    #[test]
    fn settings_save_and_load_round_trip_from_local_file() {
        let path = std::env::temp_dir().join(format!(
            "liteweight-fps-settings-{}.json",
            std::process::id()
        ));
        let settings = GameSettings {
            mouse_sensitivity: 1.7,
            ads_sensitivity: 0.55,
            fov: 90.0,
            infinite_ammo: true,
            ..GameSettings::default()
        };
        GameSettings::save_to(&path, &settings).unwrap();
        assert_eq!(GameSettings::load_from(&path), settings);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn saving_settings_replaces_an_existing_file() {
        let path = std::env::temp_dir().join(format!(
            "liteweight-fps-settings-replace-{}.json",
            std::process::id()
        ));
        let mut settings = GameSettings::default();
        GameSettings::save_to(&path, &settings).unwrap();
        settings.mouse_sensitivity = 2.0;

        GameSettings::save_to(&path, &settings).unwrap();

        assert_eq!(GameSettings::load_from(&path).mouse_sensitivity, 2.0);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn failed_atomic_write_keeps_existing_data_unchanged() {
        let directory = std::env::temp_dir().join(format!(
            "liteweight-fps-atomic-write-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let destination = directory.join("settings.json");
        fs::write(&destination, b"previous settings").unwrap();
        let blocked_temp = directory.join("blocked.tmp");
        fs::create_dir(&blocked_temp).unwrap();

        let result = super::write_atomically(&destination, &blocked_temp, b"new settings");

        assert!(result.is_err());
        assert_eq!(fs::read(&destination).unwrap(), b"previous settings");
        fs::remove_dir_all(directory).unwrap();
    }
}
