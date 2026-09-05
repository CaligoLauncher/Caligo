//! Serde types for Mojang's version manifest / version JSON / asset index,
//! plus OS rule evaluation.

use std::collections::HashMap;

use serde::Deserialize;

pub const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub const RESOURCES_URL: &str = "https://resources.download.minecraft.net";

#[derive(Deserialize)]
pub struct VersionManifest {
    pub versions: Vec<ManifestVersion>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ManifestVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
}

#[derive(Deserialize)]
pub struct VersionJson {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(default)]
    pub arguments: Option<Arguments>,
    /// Pre-1.13 versions use a flat argument string instead of `arguments`.
    #[serde(rename = "minecraftArguments", default)]
    pub minecraft_arguments: Option<String>,
    #[serde(rename = "assetIndex")]
    pub asset_index: AssetIndexRef,
    pub downloads: Downloads,
    pub libraries: Vec<Library>,
    #[serde(rename = "javaVersion", default)]
    pub java_version: Option<JavaVersion>,
}

#[derive(Deserialize)]
pub struct JavaVersion {
    #[serde(rename = "majorVersion")]
    pub major_version: u32,
}

#[derive(Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<ArgEntry>,
    #[serde(default)]
    pub jvm: Vec<ArgEntry>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ArgEntry {
    Plain(String),
    Conditional { rules: Vec<Rule>, value: ArgValue },
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ArgValue {
    One(String),
    Many(Vec<String>),
}

#[derive(Deserialize, Clone)]
pub struct Rule {
    pub action: String,
    #[serde(default)]
    pub os: Option<OsRule>,
    /// Feature-gated rules (demo mode, custom resolution, …) — we enable none.
    #[serde(default)]
    pub features: Option<serde_json::Value>,
}

#[derive(Deserialize, Clone)]
pub struct OsRule {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
}

#[derive(Deserialize)]
pub struct AssetIndexRef {
    pub id: String,
    pub sha1: String,
    pub url: String,
}

#[derive(Deserialize)]
pub struct Downloads {
    pub client: DownloadEntry,
}

#[derive(Deserialize, Clone)]
pub struct DownloadEntry {
    pub sha1: String,
    pub url: String,
}

#[derive(Deserialize, Clone)]
pub struct Library {
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
    #[serde(default)]
    pub rules: Option<Vec<Rule>>,
}

#[derive(Deserialize, Clone)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<Artifact>,
}

#[derive(Deserialize, Clone)]
pub struct Artifact {
    pub path: String,
    pub sha1: String,
    pub url: String,
}

#[derive(Deserialize)]
pub struct AssetIndexFile {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Deserialize, Clone)]
pub struct AssetObject {
    pub hash: String,
}

/// Evaluate a rule list for the current OS. Feature-gated rules never match
/// (we don't enable demo mode / custom resolution / quick play).
pub fn rules_allow(rules: &[Rule]) -> bool {
    if rules.iter().any(|r| r.features.is_some()) {
        return false;
    }
    let mut allowed = false;
    for rule in rules {
        if rule.os.as_ref().map_or(true, os_matches) {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

fn os_matches(os: &OsRule) -> bool {
    if let Some(name) = &os.name {
        let current = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "osx"
        } else {
            "linux"
        };
        if name != current {
            return false;
        }
    }
    if let Some(arch) = &os.arch {
        let current = if cfg!(target_arch = "x86") {
            "x86"
        } else if cfg!(target_arch = "x86_64") {
            "x86_64"
        } else if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "unknown"
        };
        if arch != current {
            return false;
        }
    }
    true
}
