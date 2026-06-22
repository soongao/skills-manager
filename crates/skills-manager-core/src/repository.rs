use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::model::Skill;
use crate::scan::scan_source;

pub const REPOSITORY_FILE: &str = ".skills-manager/repository.json";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositorySkill {
    pub skill_id: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryMetadata {
    pub schema_version: u32,
    pub name: String,
    pub version: u32,
    pub skills: Vec<RepositorySkill>,
}

pub fn repository_path(source_root: &Path) -> PathBuf {
    source_root.join(REPOSITORY_FILE)
}

pub fn read_repository_metadata(source_root: &Path) -> Result<Option<RepositoryMetadata>> {
    let path = repository_path(source_root);
    if !path.exists() {
        return Ok(None);
    }

    let body = fs::read_to_string(&path)
        .map_err(|err| Error::io(format!("failed to read {}", path.display()), err))?;
    let metadata: RepositoryMetadata = serde_json::from_str(&body).map_err(|err| Error::Json {
        context: format!("failed to parse {}", path.display()),
        source: err,
    })?;

    if metadata.schema_version != 1 {
        return Err(Error::InvalidInput(format!(
            "unsupported repository schema version: {}",
            metadata.schema_version
        )));
    }

    Ok(Some(metadata))
}

pub fn init_or_update_repository_metadata(
    source_root: &Path,
    name: Option<&str>,
) -> Result<RepositoryMetadata> {
    fs::create_dir_all(source_root.join("skills")).map_err(|err| {
        Error::io(
            format!("failed to create {}", source_root.join("skills").display()),
            err,
        )
    })?;

    let skills = scan_source(source_root)?;
    let existing = read_repository_metadata(source_root)?;
    let metadata = RepositoryMetadata {
        schema_version: 1,
        name: name
            .map(str::to_string)
            .or_else(|| existing.as_ref().map(|metadata| metadata.name.clone()))
            .unwrap_or_else(|| "personal-skills".to_string()),
        version: existing
            .as_ref()
            .map(|metadata| metadata.version)
            .unwrap_or(1),
        skills: skills_to_repository_skills(&skills, source_root),
    };

    write_repository_metadata(source_root, &metadata)?;
    Ok(metadata)
}

pub fn write_repository_metadata(source_root: &Path, metadata: &RepositoryMetadata) -> Result<()> {
    let path = repository_path(source_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| Error::io(format!("failed to create {}", parent.display()), err))?;
    }
    let body = serde_json::to_string_pretty(metadata).map_err(|err| Error::Json {
        context: format!("failed to serialize {}", path.display()),
        source: err,
    })?;
    fs::write(&path, format!("{body}\n"))
        .map_err(|err| Error::io(format!("failed to write {}", path.display()), err))
}

fn skills_to_repository_skills(skills: &[Skill], source_root: &Path) -> Vec<RepositorySkill> {
    skills
        .iter()
        .map(|skill| RepositorySkill {
            skill_id: skill.skill_id.clone(),
            path: skill
                .path
                .strip_prefix(source_root)
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| skill.path.clone()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn initializes_repository_metadata_from_skills() {
        let root = test_dir("repository");
        fs::create_dir_all(root.join("skills/design-clarifier")).unwrap();

        let metadata = init_or_update_repository_metadata(&root, Some("repo")).unwrap();

        assert_eq!(metadata.name, "repo");
        assert_eq!(metadata.skills.len(), 1);
        assert_eq!(
            metadata.skills[0].path,
            PathBuf::from("skills/design-clarifier")
        );
        assert!(repository_path(&root).is_file());
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
