use std::fs;
use std::path::Path;

use crate::error::{Error, Result};
use crate::model::Skill;

pub fn scan_source(source_root: &Path) -> Result<Vec<Skill>> {
    if !source_root.exists() {
        return Err(Error::SourceNotFound(source_root.display().to_string()));
    }

    let skills_root = source_root.join("skills");
    if !skills_root.is_dir() {
        return Err(Error::SourceInvalidLayout(
            source_root.display().to_string(),
        ));
    }

    let mut skills = Vec::new();
    let entries = fs::read_dir(&skills_root)
        .map_err(|err| Error::io(format!("failed to read {}", skills_root.display()), err))?;

    for entry in entries {
        let entry = entry.map_err(|err| Error::io("failed to read skills directory entry", err))?;
        let file_type = entry.file_type().map_err(|err| {
            Error::io(format!("failed to inspect {}", entry.path().display()), err)
        })?;
        if !file_type.is_dir() && !file_type.is_symlink() {
            continue;
        }

        let skill_id = entry.file_name().to_string_lossy().into_owned();
        if skill_id.starts_with('.') {
            continue;
        }

        skills.push(Skill {
            skill_id,
            path: entry.path(),
        });
    }

    skills.sort_by(|left, right| left.skill_id.cmp(&right.skill_id));
    Ok(skills)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn scans_first_level_skill_directories() {
        let root = test_dir("scan");
        fs::create_dir_all(root.join("skills/design-clarifier")).unwrap();
        fs::create_dir_all(root.join("skills/api-test")).unwrap();
        fs::write(root.join("skills/README.md"), "ignored").unwrap();

        let skills = scan_source(&root).unwrap();
        let ids: Vec<_> = skills.into_iter().map(|skill| skill.skill_id).collect();
        assert_eq!(ids, vec!["api-test", "design-clarifier"]);
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
