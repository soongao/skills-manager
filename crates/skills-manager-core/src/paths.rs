use std::env;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

pub fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
        .or_else(|| {
            let drive = env::var_os("HOMEDRIVE")?;
            let path = env::var_os("HOMEPATH")?;
            Some(PathBuf::from(format!(
                "{}{}",
                drive.to_string_lossy(),
                path.to_string_lossy()
            )))
        })
}

pub fn default_config_home() -> PathBuf {
    env::var_os("SKILLS_MANAGER_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|home| home.join(".skills-manager")))
        .unwrap_or_else(|| PathBuf::from(".skills-manager"))
}

pub fn expand_path(path: &Path, base_dir: &Path) -> Result<PathBuf> {
    let value = path.to_string_lossy();
    let expanded = expand_env_vars(&expand_home(&value));
    let expanded = PathBuf::from(expanded);
    if expanded.is_absolute() {
        Ok(expanded)
    } else {
        let base = if base_dir.as_os_str().is_empty() {
            env::current_dir()
                .map_err(|err| Error::io("failed to resolve current directory", err))?
        } else if base_dir.is_absolute() {
            base_dir.to_path_buf()
        } else {
            env::current_dir()
                .map_err(|err| Error::io("failed to resolve current directory", err))?
                .join(base_dir)
        };
        Ok(base.join(expanded))
    }
}

pub fn expand_path_from_cwd(path: &Path) -> Result<PathBuf> {
    expand_path(path, Path::new(""))
}

fn expand_home(value: &str) -> String {
    if value == "~" {
        return home_dir()
            .map(|home| home.display().to_string())
            .unwrap_or_else(|| value.to_string());
    }

    if let Some(rest) = value.strip_prefix("~/") {
        return home_dir()
            .map(|home| home.join(rest).display().to_string())
            .unwrap_or_else(|| value.to_string());
    }

    value.to_string()
}

fn expand_env_vars(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '$' {
            out.push(ch);
            continue;
        }

        if chars.peek() == Some(&'{') {
            chars.next();
            let mut name = String::new();
            for next in chars.by_ref() {
                if next == '}' {
                    break;
                }
                name.push(next);
            }
            if let Ok(value) = env::var(&name) {
                out.push_str(&value);
            } else {
                out.push_str("${");
                out.push_str(&name);
                out.push('}');
            }
            continue;
        }

        let mut name = String::new();
        while let Some(next) = chars.peek() {
            if next.is_ascii_alphanumeric() || *next == '_' {
                name.push(*next);
                chars.next();
            } else {
                break;
            }
        }

        if name.is_empty() {
            out.push('$');
        } else if let Ok(value) = env::var(&name) {
            out.push_str(&value);
        } else {
            out.push('$');
            out.push_str(&name);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_home_prefix() {
        if let Some(home) = home_dir() {
            assert_eq!(
                expand_path_from_cwd(Path::new("~/skills")).unwrap(),
                home.join("skills")
            );
        }
    }
}
