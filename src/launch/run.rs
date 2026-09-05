//! Building the Java command line (JVM + game args) and finding Java.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::install::Prepared;
use super::manifest::{rules_allow, ArgEntry, ArgValue};

pub struct LaunchProfile {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
}

pub fn build_command(prepared: &Prepared, profile: &LaunchProfile) -> Result<Command, String> {
    let dir = &prepared.game_dir;
    let natives = dir.join("natives");
    std::fs::create_dir_all(&natives).map_err(|e| format!("Создание папок: {e}"))?;

    let sep = if cfg!(windows) { ";" } else { ":" };
    let classpath = prepared
        .classpath
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(sep);

    let v = &prepared.version_json;
    let vars: Vec<(&str, String)> = vec![
        ("${auth_player_name}", profile.username.clone()),
        ("${version_name}", v.id.clone()),
        ("${game_directory}", dir.display().to_string()),
        ("${assets_root}", dir.join("assets").display().to_string()),
        ("${assets_index_name}", v.asset_index.id.clone()),
        ("${auth_uuid}", profile.uuid.clone()),
        ("${auth_access_token}", profile.access_token.clone()),
        ("${auth_xuid}", String::new()),
        ("${clientid}", String::new()),
        ("${user_type}", "msa".into()),
        ("${version_type}", v.kind.clone()),
        ("${natives_directory}", natives.display().to_string()),
        ("${launcher_name}", "TerraLauncher".into()),
        ("${launcher_version}", env!("CARGO_PKG_VERSION").into()),
        ("${classpath}", classpath.clone()),
    ];
    let subst = |s: &str| {
        let mut out = s.to_string();
        for (key, value) in &vars {
            out = out.replace(key, value);
        }
        out
    };

    let mut jvm_args: Vec<String> = vec!["-Xmx2G".into()];
    let mut game_args: Vec<String> = Vec::new();
    if let Some(arguments) = &v.arguments {
        for entry in &arguments.jvm {
            push_arg(&mut jvm_args, entry, &subst);
        }
        for entry in &arguments.game {
            push_arg(&mut game_args, entry, &subst);
        }
    } else {
        // Pre-1.13 layout.
        jvm_args.push(format!("-Djava.library.path={}", natives.display()));
        jvm_args.push("-cp".into());
        jvm_args.push(classpath.clone());
        if let Some(legacy) = &v.minecraft_arguments {
            game_args.extend(legacy.split_whitespace().map(&subst));
        }
    }

    let mut cmd = Command::new(find_java());
    cmd.args(&jvm_args)
        .arg(&v.main_class)
        .args(&game_args)
        .current_dir(dir);
    Ok(cmd)
}

fn push_arg(out: &mut Vec<String>, entry: &ArgEntry, subst: &impl Fn(&str) -> String) {
    match entry {
        ArgEntry::Plain(s) => out.push(subst(s)),
        ArgEntry::Conditional { rules, value } => {
            if rules_allow(rules) {
                match value {
                    ArgValue::One(s) => out.push(subst(s)),
                    ArgValue::Many(many) => out.extend(many.iter().map(|s| subst(s))),
                }
            }
        }
    }
}

/// JAVA_HOME first, then javaw/java from PATH.
fn find_java() -> PathBuf {
    let exe = if cfg!(windows) { "javaw.exe" } else { "java" };
    if let Ok(home) = std::env::var("JAVA_HOME") {
        let p = Path::new(&home).join("bin").join(exe);
        if p.exists() {
            return p;
        }
    }
    PathBuf::from(if cfg!(windows) { "javaw" } else { "java" })
}
