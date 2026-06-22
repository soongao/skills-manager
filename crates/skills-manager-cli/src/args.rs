use std::collections::BTreeMap;
use std::path::PathBuf;

use skills_manager_core::paths::default_config_home;

#[derive(Clone, Debug)]
pub struct CliArgs {
    pub command: Vec<String>,
    options: BTreeMap<String, String>,
    flags: Vec<String>,
    pub json: bool,
}

impl CliArgs {
    pub fn parse(raw: Vec<String>) -> Self {
        let mut command = Vec::new();
        let mut options = BTreeMap::new();
        let mut flags = Vec::new();
        let mut json = false;
        let mut index = 0;

        while index < raw.len() {
            let item = &raw[index];
            if item == "--json" {
                json = true;
                index += 1;
                continue;
            }

            if let Some(name) = item.strip_prefix("--") {
                if index + 1 < raw.len() && !raw[index + 1].starts_with("--") {
                    options.insert(name.to_string(), raw[index + 1].clone());
                    index += 2;
                } else {
                    flags.push(name.to_string());
                    index += 1;
                }
                continue;
            }

            command.push(item.clone());
            index += 1;
        }

        Self {
            command,
            options,
            flags,
            json,
        }
    }

    pub fn option(&self, name: &str) -> Option<String> {
        self.options.get(name).cloned()
    }

    pub fn flag(&self, name: &str) -> bool {
        self.flags.iter().any(|flag| flag == name)
    }

    pub fn config_home(&self) -> PathBuf {
        self.option("config-home")
            .map(PathBuf::from)
            .unwrap_or_else(default_config_home)
    }
}
