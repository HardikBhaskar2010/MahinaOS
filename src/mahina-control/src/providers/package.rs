use crate::error::ControlError;
use crate::protocol::PackageEntry;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const DEFAULT_LPKG_DB_PATH: &str = "/var/lib/lpkg/installed.toml";
pub const FALLBACK_LPKG_DB_PATH: &str = "/var/lib/lpkg/installed.db";

#[derive(Debug, Clone)]
pub struct PackageProvider {
    db_path: PathBuf,
}

impl PackageProvider {
    pub fn new() -> Self {
        Self {
            db_path: PathBuf::from(DEFAULT_LPKG_DB_PATH),
        }
    }

    pub fn with_db_path(path: impl AsRef<Path>) -> Self {
        Self {
            db_path: path.as_ref().to_path_buf(),
        }
    }

    pub fn list(&self) -> Result<Vec<PackageEntry>, ControlError> {
        let fallback = PathBuf::from(FALLBACK_LPKG_DB_PATH);
        let active_path = if self.db_path.exists() {
            &self.db_path
        } else if fallback.exists() {
            &fallback
        } else {
            return Ok(Vec::new());
        };

        let file = match File::open(active_path) {
            Ok(f) => f,
            Err(_) => return Ok(Vec::new()),
        };

        let reader = BufReader::new(file);
        let mut pkgs = Vec::new();

        let mut cur_name = String::new();
        let mut cur_ver = String::new();
        let mut cur_desc = String::new();
        let mut cur_files = 0;
        let mut inside_pkg = false;

        for line_res in reader.lines() {
            let line = match line_res {
                Ok(l) => l,
                Err(_) => continue,
            };
            let trimmed = line.trim();

            if trimmed == "[[package]]" {
                if inside_pkg && !cur_name.is_empty() {
                    pkgs.push(PackageEntry {
                        name: cur_name.clone(),
                        version: cur_ver.clone(),
                        description: cur_desc.clone(),
                        file_count: cur_files,
                    });
                }
                cur_name.clear();
                cur_ver.clear();
                cur_desc.clear();
                cur_files = 0;
                inside_pkg = true;
            } else if inside_pkg {
                if let Some(rest) = trimmed.strip_prefix("name = \"") {
                    if let Some(val) = rest.strip_suffix('"') {
                        cur_name = val.to_string();
                    }
                } else if let Some(rest) = trimmed.strip_prefix("version = \"") {
                    if let Some(val) = rest.strip_suffix('"') {
                        cur_ver = val.to_string();
                    }
                } else if let Some(rest) = trimmed.strip_prefix("description = \"") {
                    if let Some(val) = rest.strip_suffix('"') {
                        cur_desc = val.to_string();
                    }
                } else if trimmed.starts_with("\"/") {
                    cur_files += 1;
                }
            }
        }

        if inside_pkg && !cur_name.is_empty() {
            pkgs.push(PackageEntry {
                name: cur_name,
                version: cur_ver,
                description: cur_desc,
                file_count: cur_files,
            });
        }

        Ok(pkgs)
    }

    pub fn install(&self, target: &str) -> Result<String, ControlError> {
        if target.trim().is_empty() {
            return Err(ControlError::InvalidParameter {
                param: "target".to_string(),
                reason: "Package target name or path cannot be empty".to_string(),
            });
        }

        let output = Command::new("lpkg")
            .arg("install")
            .arg(target)
            .output()
            .map_err(|e| ControlError::Io(format!("Failed to execute 'lpkg install {}': {}", target, e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ControlError::Internal(format!(
                "lpkg install failed with code {}: {}",
                output.status.code().unwrap_or(-1),
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(if stdout.trim().is_empty() {
            format!("Package '{}' installed successfully", target)
        } else {
            stdout.trim().to_string()
        })
    }

    pub fn remove(&self, name: &str) -> Result<String, ControlError> {
        if name.trim().is_empty() {
            return Err(ControlError::InvalidParameter {
                param: "name".to_string(),
                reason: "Package name cannot be empty".to_string(),
            });
        }

        let output = Command::new("lpkg")
            .arg("remove")
            .arg(name)
            .output()
            .map_err(|e| ControlError::Io(format!("Failed to execute 'lpkg remove {}': {}", name, e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ControlError::Internal(format!(
                "lpkg remove failed with code {}: {}",
                output.status.code().unwrap_or(-1),
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(if stdout.trim().is_empty() {
            format!("Package '{}' removed successfully", name)
        } else {
            stdout.trim().to_string()
        })
    }
}

impl Default for PackageProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_package_provider_parser() {
        let toml_content = r#"
# /var/lib/lpkg/installed.toml — Managed by lpkg

[[package]]
name = "nano"
version = "7.2"
description = "GNU nano editor"
files = [
  "/bin/nano",
  "/usr/share/nano/syntax.nanorc"
]

[[package]]
name = "curl"
version = "8.4.0"
description = "Command line tool for transferring data with URL syntax"
files = [
  "/usr/bin/curl"
]
"#;

        let temp_file = std::env::temp_dir().join("test_installed.toml");
        {
            let mut f = File::create(&temp_file).unwrap();
            f.write_all(toml_content.as_bytes()).unwrap();
        }

        let provider = PackageProvider::with_db_path(&temp_file);
        let pkgs = provider.list().expect("list failed");

        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].name, "nano");
        assert_eq!(pkgs[0].version, "7.2");
        assert_eq!(pkgs[0].description, "GNU nano editor");
        assert_eq!(pkgs[0].file_count, 2);

        assert_eq!(pkgs[1].name, "curl");
        assert_eq!(pkgs[1].version, "8.4.0");
        assert_eq!(pkgs[1].file_count, 1);

        let _ = std::fs::remove_file(&temp_file);
    }
}
