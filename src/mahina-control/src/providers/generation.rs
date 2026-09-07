use crate::error::ControlError;
use crate::protocol::GenerationEntry;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub const DEFAULT_GENERATIONS_DIR: &str = "/var/lib/mahina/generations";
pub const FALLBACK_GENERATIONS_DIR: &str = "/@generations";
pub const DEFAULT_BASELINE_GEN_ID: u32 = 101;

#[derive(Debug, Clone)]
pub struct GenerationProvider {
    generations_dir: PathBuf,
}

impl GenerationProvider {
    pub fn new() -> Self {
        Self {
            generations_dir: PathBuf::from(DEFAULT_GENERATIONS_DIR),
        }
    }

    pub fn with_dir(dir: impl AsRef<Path>) -> Self {
        Self {
            generations_dir: dir.as_ref().to_path_buf(),
        }
    }

    pub fn get_current(&self) -> Result<GenerationEntry, ControlError> {
        let list = self.list()?;
        if let Some(cur) = list.into_iter().find(|g| g.is_current) {
            Ok(cur)
        } else {
            // Fallback baseline generation
            Ok(GenerationEntry {
                id: DEFAULT_BASELINE_GEN_ID,
                description: "MahinaOS Baseline System Generation".to_string(),
                created_at_epoch_secs: 1788739200, // 2026 baseline epoch
                is_current: true,
                is_healthy: true,
                kernel_version: "Linux 6.6-mahina".to_string(),
            })
        }
    }
    pub fn list(&self) -> Result<Vec<GenerationEntry>, ControlError> {
        let fallback_dir = PathBuf::from(FALLBACK_GENERATIONS_DIR);
        let active_dir: &Path = if self.generations_dir.exists() {
            &self.generations_dir
        } else if fallback_dir.exists() {
            &fallback_dir
        } else {
            // If generations directory is not yet populated, return baseline generation
            return Ok(vec![GenerationEntry {
                id: DEFAULT_BASELINE_GEN_ID,
                description: "MahinaOS Baseline System Generation".to_string(),
                created_at_epoch_secs: 1788739200,
                is_current: true,
                is_healthy: true,
                kernel_version: "Linux 6.6-mahina".to_string(),
            }]);
        };

        // Check if there is a 'current' symlink
        let current_gen_id = fs::read_link(active_dir.join("current"))
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .and_then(|name| name.parse::<u32>().ok())
            .unwrap_or(DEFAULT_BASELINE_GEN_ID);

        let mut generations = Vec::new();

        if let Ok(entries) = fs::read_dir(active_dir) {
            for entry_res in entries {
                let entry = match entry_res {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                let file_name = entry.file_name().to_string_lossy().to_string();
                if let Ok(gen_id) = file_name.parse::<u32>() {
                    let path = entry.path();
                    let metadata = fs::metadata(&path).ok();
                    let created_secs = metadata
                        .and_then(|m| m.created().or_else(|_| m.modified()).ok())
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(1788739200);

                    let is_current = gen_id == current_gen_id;
                    let desc = if gen_id == DEFAULT_BASELINE_GEN_ID {
                        "MahinaOS Baseline System Generation".to_string()
                    } else {
                        format!("Generation #{}", gen_id)
                    };

                    generations.push(GenerationEntry {
                        id: gen_id,
                        description: desc,
                        created_at_epoch_secs: created_secs,
                        is_current,
                        is_healthy: true,
                        kernel_version: "Linux 6.6-mahina".to_string(),
                    });
                }
            }
        }

        if generations.is_empty() {
            generations.push(GenerationEntry {
                id: DEFAULT_BASELINE_GEN_ID,
                description: "MahinaOS Baseline System Generation".to_string(),
                created_at_epoch_secs: 1788739200,
                is_current: true,
                is_healthy: true,
                kernel_version: "Linux 6.6-mahina".to_string(),
            });
        }

        generations.sort_by_key(|g| g.id);
        Ok(generations)
    }
}

impl Default for GenerationProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generation_provider_baseline() {
        let provider = GenerationProvider::with_dir("/tmp/nonexistent-generations-dir");
        let current = provider.get_current().expect("get_current failed");
        assert_eq!(current.id, DEFAULT_BASELINE_GEN_ID);
        assert!(current.is_current);
        assert!(current.is_healthy);

        let list = provider.list().expect("list failed");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, DEFAULT_BASELINE_GEN_ID);
    }
}
