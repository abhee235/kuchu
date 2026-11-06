//! Kuchu's settings, kept in one small file in the app's config folder and sent to every window when they change.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Prefs {
    /// Which pet it is: "robot", "cat", "dog", "owl", "frog" or "duck".
    pub pet: String,
    /// "small" (64 px), "base" (96) or "big" (128).
    pub size: String,
    /// The pet fades while the pointer is on it.
    pub see_through: bool,
    /// What shows beside the pet: "line", "lines" or "off".
    pub card: String,
    /// An answer stays beside the pet until it is clicked; otherwise it shows for a few seconds.
    pub keep_answer: bool,
    pub sounds: bool,
    /// What a click on the pet opens: "sessions" or "settings".
    pub click: String,
    /// Permission requests are shown on Kuchu, with Allow and Deny.
    pub approvals: bool,
    /// With no work, Kuchu plays now and then (stretches, yawns, strolls) and naps after a long quiet.
    pub lively: bool,
    /// Where the pet was left (its box's top-left, physical px), so it comes back there.
    pub at: Option<(i32, i32)>,
    /// Folders conversations were started in from Kuchu, the latest first.
    pub folders: Vec<String>,
}

/// How many recent folders are kept.
const FOLDERS: usize = 6;

impl Default for Prefs {
    fn default() -> Self {
        Prefs { pet: "robot".into(), size: "base".into(), see_through: true, card: "line".into(), keep_answer: false, sounds: true, click: "sessions".into(), approvals: true, lively: true, at: None, folders: Vec::new() }
    }
}

impl Prefs {
    pub fn width(&self) -> u32 {
        match self.size.as_str() {
            "small" => 64,
            "big" => 128,
            _ => 96,
        }
    }

    /// Unknown values from an older or hand-edited file fall back to the defaults.
    fn checked(mut self) -> Self {
        let d = Prefs::default();
        if !matches!(self.pet.as_str(), "robot" | "cat" | "dog" | "owl" | "frog" | "duck") {
            self.pet = d.pet;
        }
        if !matches!(self.size.as_str(), "small" | "base" | "big") {
            self.size = d.size;
        }
        if !matches!(self.card.as_str(), "line" | "lines" | "off") {
            self.card = d.card;
        }
        if !matches!(self.click.as_str(), "sessions" | "settings") {
            self.click = d.click;
        }
        self.folders.truncate(FOLDERS);
        self
    }

    /// A folder a conversation was just started in goes first.
    pub fn used_folder(&mut self, folder: &str) {
        self.folders.retain(|f| !f.eq_ignore_ascii_case(folder));
        self.folders.insert(0, folder.to_string());
        self.folders.truncate(FOLDERS);
    }

    pub fn load(path: &Path) -> Prefs {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Prefs>(&text).ok())
            .map(Prefs::checked)
            .unwrap_or_default()
    }

    /// Written beside and renamed over, so a crash never leaves half a file.
    pub fn save(&self, path: &Path) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = path.with_extension("json.tmp");
        if let Ok(text) = serde_json::to_string_pretty(self) {
            if std::fs::write(&tmp, text).is_ok() {
                let _ = std::fs::rename(&tmp, path);
            }
        }
    }

    /// Applies a change from a window: only the fields it names.
    pub fn merged(&self, patch: &serde_json::Value) -> Prefs {
        let mut whole = serde_json::to_value(self).unwrap_or_default();
        if let (Some(base), Some(changes)) = (whole.as_object_mut(), patch.as_object()) {
            for (k, v) in changes {
                if base.contains_key(k) {
                    base.insert(k.clone(), v.clone());
                }
            }
        }
        serde_json::from_value::<Prefs>(whole).map(Prefs::checked).unwrap_or_else(|_| self.clone())
    }
}

pub fn path_in(config_dir: PathBuf) -> PathBuf {
    config_dir.join("settings.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_touches_only_what_it_names_and_bad_values_fall_back() {
        let p = Prefs::default();
        let q = p.merged(&serde_json::json!({ "size": "big", "sounds": false, "nonsense": 1 }));
        assert_eq!((q.width(), q.sounds, q.card.as_str()), (128, false, "line"));
        let r = q.merged(&serde_json::json!({ "card": "sideways" }));
        assert_eq!(r.card, "line");
    }

    #[test]
    fn the_chosen_pet_is_kept_and_a_file_from_before_pets_or_after_more_of_them_gives_the_robot() {
        assert_eq!(Prefs::default().merged(&serde_json::json!({ "pet": "cat" })).pet, "cat");
        assert_eq!(Prefs::default().merged(&serde_json::json!({ "pet": "dragon" })).pet, "robot");

        let dir = std::env::temp_dir().join(format!("kuchu-prefs-pet-{}", std::process::id()));
        let path = path_in(dir.clone());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, r#"{ "size": "big" }"#).unwrap();
        let older = Prefs::load(&path);
        assert_eq!((older.pet.as_str(), older.width()), ("robot", 128));
        std::fs::write(&path, r#"{ "pet": "dragon", "size": "big" }"#).unwrap();
        let newer = Prefs::load(&path);
        assert_eq!((newer.pet.as_str(), newer.width()), ("robot", 128));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_missing_or_broken_file_gives_the_defaults() {
        let dir = std::env::temp_dir().join(format!("kuchu-prefs-{}", std::process::id()));
        let path = path_in(dir.clone());
        assert_eq!(Prefs::load(&path), Prefs::default());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(Prefs::load(&path), Prefs::default());
        let mut p = Prefs::default();
        p.size = "small".into();
        p.save(&path);
        assert_eq!(Prefs::load(&path).width(), 64);
        let _ = std::fs::remove_dir_all(dir);
    }
}
