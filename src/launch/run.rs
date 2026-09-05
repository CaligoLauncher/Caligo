//! Building the Java command line (JVM + game args) and finding Java.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::install::Prepared;
use super::manifest::{rules_allow, ArgEntry, ArgValue};

pub struct LaunchProfile {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
}

/// Что за Java установлена у пользователя.
pub struct JavaInfo {
    pub path: PathBuf,
    pub major: u32,
    pub is_64bit: bool,
    pub version_line: String,
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
        ("${launcher_name}", "Caligo".into()),
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
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    hide_console(&mut cmd);
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

/// JAVA_HOME first, then java from PATH. Мы всегда используем java.exe
/// (не javaw): консольное окно прячем флагом CREATE_NO_WINDOW, зато
/// stdout/stderr можно перехватить и показать настоящую ошибку.
fn find_java() -> PathBuf {
    let exe = if cfg!(windows) { "java.exe" } else { "java" };
    if let Ok(home) = std::env::var("JAVA_HOME") {
        let p = Path::new(&home).join("bin").join(exe);
        if p.exists() {
            return p;
        }
    }
    PathBuf::from("java")
}

/// Не показывать чёрное консольное окно на Windows.
fn hide_console(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        let _ = cmd;
    }
}

/// Запускает `java -version` и разбирает, что за Java установлена.
pub fn check_java() -> Result<JavaInfo, String> {
    let path = find_java();
    let mut cmd = Command::new(&path);
    cmd.arg("-version").stdout(Stdio::piped()).stderr(Stdio::piped());
    hide_console(&mut cmd);
    let out = cmd.output().map_err(|e| {
        format!("Java не найдена ({e}). Установи Java 21 (например, Temurin JDK) или задай JAVA_HOME")
    })?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let version_line = text.lines().next().unwrap_or("").trim().to_string();
    let major = parse_major(&text)
        .ok_or_else(|| format!("Не удалось разобрать версию Java: {version_line}"))?;
    let is_64bit = text.contains("64-Bit");
    Ok(JavaInfo {
        path,
        major,
        is_64bit,
        version_line,
    })
}

/// Первый токен в кавычках: `"21.0.2"` -> 21, `"1.8.0_391"` -> 8, `"17"` -> 17.
fn parse_major(text: &str) -> Option<u32> {
    let start = text.find('"')? + 1;
    let end = start + text[start..].find('"')?;
    let ver = &text[start..end];
    let mut parts = ver.split(['.', '_', '-', '+']);
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

/// Читает поток построчно в фоне, храня последние `keep` строк.
pub fn spawn_tail_reader<R: std::io::Read + Send + 'static>(
    reader: Option<R>,
    keep: usize,
) -> std::thread::JoinHandle<Vec<String>> {
    std::thread::spawn(move || {
        let mut tail: VecDeque<String> = VecDeque::new();
        if let Some(r) = reader {
            for line in BufReader::new(r).lines().map_while(Result::ok) {
                if tail.len() >= keep {
                    tail.pop_front();
                }
                tail.push_back(line);
            }
        }
        tail.into_iter().collect()
    })
}

#[cfg(test)]
mod tests {
    use super::parse_major;

    #[test]
    fn parses_modern_and_legacy_versions() {
        assert_eq!(parse_major(r#"openjdk version "21.0.2" 2024-01-16"#), Some(21));
        assert_eq!(parse_major(r#"java version "1.8.0_391""#), Some(8));
        assert_eq!(parse_major(r#"openjdk version "17" 2021-09-14"#), Some(17));
    }
}
