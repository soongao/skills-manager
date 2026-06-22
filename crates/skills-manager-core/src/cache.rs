use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

pub const CACHE_MARKER_FILE: &str = ".skills-manager-cache.json";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheMarker {
    pub repo_id: String,
    pub source_profile_id: String,
}

impl CacheMarker {
    pub fn new(repo_id: impl Into<String>, source_profile_id: impl Into<String>) -> Self {
        Self {
            repo_id: repo_id.into(),
            source_profile_id: source_profile_id.into(),
        }
    }
}

pub fn marker_path(cache_root: &Path) -> PathBuf {
    cache_root.join(CACHE_MARKER_FILE)
}

pub fn init_cache_marker(cache_root: &Path, marker: &CacheMarker) -> Result<()> {
    fs::create_dir_all(cache_root)
        .map_err(|err| Error::io(format!("failed to create {}", cache_root.display()), err))?;
    let body = format!(
        concat!(
            "{{\n",
            "  \"schemaVersion\": 1,\n",
            "  \"managedBy\": \"skills-manager\",\n",
            "  \"repoId\": \"{}\",\n",
            "  \"sourceProfileId\": \"{}\"\n",
            "}}\n"
        ),
        escape_json_string(&marker.repo_id),
        escape_json_string(&marker.source_profile_id)
    );
    fs::write(marker_path(cache_root), body).map_err(|err| {
        Error::io(
            format!("failed to write cache marker in {}", cache_root.display()),
            err,
        )
    })
}

pub fn verify_cache_marker(cache_root: &Path, expected: &CacheMarker) -> Result<()> {
    let path = marker_path(cache_root);
    if !path.is_file() {
        return Err(Error::CacheMarkerMissing(path.display().to_string()));
    }

    let body = fs::read_to_string(&path)
        .map_err(|err| Error::io(format!("failed to read {}", path.display()), err))?;
    let repo_id = extract_json_string_field(&body, "repoId")
        .ok_or_else(|| Error::CacheMarkerMismatch(path.display().to_string()))?;
    let source_profile_id = extract_json_string_field(&body, "sourceProfileId")
        .ok_or_else(|| Error::CacheMarkerMismatch(path.display().to_string()))?;
    let managed_by = extract_json_string_field(&body, "managedBy")
        .ok_or_else(|| Error::CacheMarkerMismatch(path.display().to_string()))?;

    if managed_by != "skills-manager"
        || repo_id != expected.repo_id
        || source_profile_id != expected.source_profile_id
    {
        return Err(Error::CacheMarkerMismatch(path.display().to_string()));
    }

    Ok(())
}

fn escape_json_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn extract_json_string_field(body: &str, field: &str) -> Option<String> {
    let key = format!("\"{field}\"");
    let key_start = body.find(&key)?;
    let after_key = &body[key_start + key.len()..];
    let colon_index = after_key.find(':')?;
    let mut value = after_key[colon_index + 1..].trim_start();
    if !value.starts_with('"') {
        return None;
    }
    value = &value[1..];

    let mut out = String::new();
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => return Some(out),
            _ => out.push(ch),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn verifies_matching_marker() {
        let root = test_dir("cache-marker");
        let marker = CacheMarker::new("repo", "source");
        init_cache_marker(&root, &marker).unwrap();
        verify_cache_marker(&root, &marker).unwrap();
    }

    #[test]
    fn rejects_mismatched_marker() {
        let root = test_dir("cache-marker-mismatch");
        init_cache_marker(&root, &CacheMarker::new("repo", "source")).unwrap();
        let err = verify_cache_marker(&root, &CacheMarker::new("other", "source")).unwrap_err();
        assert!(matches!(err, Error::CacheMarkerMismatch(_)));
    }

    fn test_dir(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("skills-manager-{name}-{nanos}"));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
