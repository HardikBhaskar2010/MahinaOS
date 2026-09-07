/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * state.rs — MahinaOS State Architecture & Generation Registry.
 *            Manages the isolated state store at `/system/mahina-state/` (Btrfs @state),
 *            the single-writer GenerationRegistry, GenerationManifest, and transactional
 *            BootState atomic records.
 */

use crate::error::ControlError;
use crate::sha256::Sha256;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const DEFAULT_STATE_DIR: &str = "/system/mahina-state";
pub const DEFAULT_BASELINE_GEN_ID: u32 = 101;
pub const DEFAULT_RETENTION_QUOTA: usize = 5;
pub const DEFAULT_MAX_BOOT_ATTEMPTS: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationStatus {
    Staged,
    Candidate,
    Booted,
    Healthy,
    Superseded,
    Failed,
    Archived,
    Pruned,
}

impl std::fmt::Display for GenerationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenerationStatus::Staged => write!(f, "STAGED"),
            GenerationStatus::Candidate => write!(f, "CANDIDATE"),
            GenerationStatus::Booted => write!(f, "BOOTED"),
            GenerationStatus::Healthy => write!(f, "HEALTHY"),
            GenerationStatus::Superseded => write!(f, "SUPERSEDED"),
            GenerationStatus::Failed => write!(f, "FAILED"),
            GenerationStatus::Archived => write!(f, "ARCHIVED"),
            GenerationStatus::Pruned => write!(f, "PRUNED"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BootStage {
    NormalBoot,
    CandidateTrial,
    FailedRollback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BootHealthStatus {
    Pending,
    Healthy,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenerationManifest {
    pub id: u32,
    pub parent_id: Option<u32>,
    pub description: String,
    pub created_at_epoch_secs: u64,
    pub kernel_version: String,
    pub status: GenerationStatus,
    pub btrfs_tree_root_id: u64,
    pub is_pinned: bool,
    pub commit_digest_sha256: String,
}

impl GenerationManifest {
    /// Calculate canonical commit digest excluding the `commit_digest_sha256` field.
    pub fn compute_digest(
        id: u32,
        parent_id: Option<u32>,
        description: &str,
        created_at_epoch_secs: u64,
        kernel_version: &str,
        status: GenerationStatus,
        btrfs_tree_root_id: u64,
        is_pinned: bool,
        package_manifest: &str,
        kernel_sha256: &str,
        initramfs_sha256: &str,
    ) -> String {
        let parent_str = match parent_id {
            Some(p) => p.to_string(),
            None => "none".to_string(),
        };
        let preimage = format!(
            "id:{}|parent:{}|desc:{}|time:{}|kernel:{}|status:{}|btrfs_tree:{}|pinned:{}|pkg_digest:{}|k_hash:{}|initrd_hash:{}",
            id,
            parent_str,
            description,
            created_at_epoch_secs,
            kernel_version,
            status,
            btrfs_tree_root_id,
            is_pinned,
            Sha256::digest_hex(package_manifest.as_bytes()),
            kernel_sha256,
            initramfs_sha256
        );
        Sha256::digest_hex(preimage.as_bytes())
    }

    pub fn verify_digest(
        &self,
        package_manifest: &str,
        kernel_sha256: &str,
        initramfs_sha256: &str,
    ) -> bool {
        let expected = Self::compute_digest(
            self.id,
            self.parent_id,
            &self.description,
            self.created_at_epoch_secs,
            &self.kernel_version,
            self.status,
            self.btrfs_tree_root_id,
            self.is_pinned,
            package_manifest,
            kernel_sha256,
            initramfs_sha256,
        );
        self.commit_digest_sha256 == expected
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenerationSummary {
    pub id: u32,
    pub parent_id: Option<u32>,
    pub description: String,
    pub created_at_epoch_secs: u64,
    pub status: GenerationStatus,
    pub is_pinned: bool,
    pub is_current: bool,
    pub is_fallback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenerationRegistry {
    pub current_generation_id: u32,
    pub fallback_generation_id: u32,
    pub next_generation_id: u32,
    pub retention_quota: usize,
    pub pinned_generations: Vec<u32>,
    pub generations: Vec<GenerationSummary>,
}

impl GenerationRegistry {
    pub fn baseline() -> Self {
        Self {
            current_generation_id: DEFAULT_BASELINE_GEN_ID,
            fallback_generation_id: DEFAULT_BASELINE_GEN_ID,
            next_generation_id: DEFAULT_BASELINE_GEN_ID + 1,
            retention_quota: DEFAULT_RETENTION_QUOTA,
            pinned_generations: vec![DEFAULT_BASELINE_GEN_ID],
            generations: vec![GenerationSummary {
                id: DEFAULT_BASELINE_GEN_ID,
                parent_id: None,
                description: "MahinaOS Baseline System Generation".to_string(),
                created_at_epoch_secs: 1788739200,
                status: GenerationStatus::Healthy,
                is_pinned: true,
                is_current: true,
                is_fallback: true,
            }],
        }
    }

    pub fn get_summary(&self, id: u32) -> Option<&GenerationSummary> {
        self.generations.iter().find(|g| g.id == id)
    }

    pub fn is_pinned(&self, id: u32) -> bool {
        self.pinned_generations.contains(&id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootState {
    pub generation_id: u32,
    pub boot_attempts: u32,
    pub max_attempts: u32,
    pub boot_id: String,
    pub stage: BootStage,
    pub fallback_generation_id: u32,
    pub health_status: BootHealthStatus,
}

impl BootState {
    pub fn new(generation_id: u32, fallback_id: u32) -> Self {
        Self {
            generation_id,
            boot_attempts: 0,
            max_attempts: DEFAULT_MAX_BOOT_ATTEMPTS,
            boot_id: "boot-init".to_string(),
            stage: BootStage::NormalBoot,
            fallback_generation_id: fallback_id,
            health_status: BootHealthStatus::Healthy,
        }
    }

    pub fn candidate_trial(candidate_id: u32, fallback_id: u32, boot_id: String) -> Self {
        Self {
            generation_id: candidate_id,
            boot_attempts: 0,
            max_attempts: DEFAULT_MAX_BOOT_ATTEMPTS,
            boot_id,
            stage: BootStage::CandidateTrial,
            fallback_generation_id: fallback_id,
            health_status: BootHealthStatus::Pending,
        }
    }

    pub fn to_record(&self) -> String {
        let stage_str = match self.stage {
            BootStage::NormalBoot => "NORMAL_BOOT",
            BootStage::CandidateTrial => "CANDIDATE_TRIAL",
            BootStage::FailedRollback => "FAILED_ROLLBACK",
        };
        let health_str = match self.health_status {
            BootHealthStatus::Pending => "PENDING",
            BootHealthStatus::Healthy => "HEALTHY",
            BootHealthStatus::Failed => "FAILED",
        };

        format!(
            "# MahinaOS Transactional Boot State - Managed by mahina-control-d\n\
             GENERATION_ID={}\n\
             BOOT_ATTEMPTS={}\n\
             MAX_ATTEMPTS={}\n\
             BOOT_ID={}\n\
             STAGE={}\n\
             FALLBACK_GENERATION_ID={}\n\
             HEALTH_STATUS={}\n",
            self.generation_id,
            self.boot_attempts,
            self.max_attempts,
            self.boot_id,
            stage_str,
            self.fallback_generation_id,
            health_str
        )
    }

    pub fn parse(content: &str) -> Result<Self, ControlError> {
        let mut gen_id = None;
        let mut attempts = None;
        let mut max_attempts = None;
        let mut boot_id = None;
        let mut stage = None;
        let mut fallback_id = None;
        let mut health = None;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.splitn(2, '=');
            let key = parts.next().unwrap_or("").trim();
            let val = parts.next().unwrap_or("").trim();

            match key {
                "GENERATION_ID" => {
                    gen_id = val.parse::<u32>().ok();
                }
                "BOOT_ATTEMPTS" => {
                    attempts = val.parse::<u32>().ok();
                }
                "MAX_ATTEMPTS" => {
                    max_attempts = val.parse::<u32>().ok();
                }
                "BOOT_ID" => {
                    boot_id = Some(val.to_string());
                }
                "STAGE" => {
                    stage = match val {
                        "NORMAL_BOOT" => Some(BootStage::NormalBoot),
                        "CANDIDATE_TRIAL" => Some(BootStage::CandidateTrial),
                        "FAILED_ROLLBACK" => Some(BootStage::FailedRollback),
                        _ => None,
                    };
                }
                "FALLBACK_GENERATION_ID" => {
                    fallback_id = val.parse::<u32>().ok();
                }
                "HEALTH_STATUS" => {
                    health = match val {
                        "PENDING" => Some(BootHealthStatus::Pending),
                        "HEALTHY" => Some(BootHealthStatus::Healthy),
                        "FAILED" => Some(BootHealthStatus::Failed),
                        _ => None,
                    };
                }
                _ => {}
            }
        }

        Ok(Self {
            generation_id: gen_id.ok_or_else(|| ControlError::InvalidParameter {
                param: "GENERATION_ID".to_string(),
                reason: "Missing GENERATION_ID in boot-state".to_string(),
            })?,
            boot_attempts: attempts.unwrap_or(0),
            max_attempts: max_attempts.unwrap_or(DEFAULT_MAX_BOOT_ATTEMPTS),
            boot_id: boot_id.unwrap_or_else(|| "unknown".to_string()),
            stage: stage.unwrap_or(BootStage::NormalBoot),
            fallback_generation_id: fallback_id.unwrap_or(DEFAULT_BASELINE_GEN_ID),
            health_status: health.unwrap_or(BootHealthStatus::Healthy),
        })
    }

    /// Record next boot attempt according to exact 0..3 semantics.
    /// Returns true if attempt count exceeded max_attempts triggering automated rollback.
    pub fn record_attempt(&mut self) -> bool {
        self.boot_attempts = self.boot_attempts.saturating_add(1);
        if self.boot_attempts > self.max_attempts {
            self.stage = BootStage::FailedRollback;
            self.generation_id = self.fallback_generation_id;
            self.health_status = BootHealthStatus::Failed;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct StateStore {
    root_dir: PathBuf,
}

impl Default for StateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl StateStore {
    pub fn new() -> Self {
        Self {
            root_dir: PathBuf::from(DEFAULT_STATE_DIR),
        }
    }

    pub fn with_dir(path: impl AsRef<Path>) -> Self {
        Self {
            root_dir: path.as_ref().to_path_buf(),
        }
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn db_dir(&self) -> PathBuf {
        self.root_dir.join("db")
    }

    pub fn boot_dir(&self) -> PathBuf {
        self.root_dir.join("boot")
    }

    pub fn registry_path(&self) -> PathBuf {
        self.db_dir().join("registry.json")
    }

    pub fn boot_state_path(&self) -> PathBuf {
        self.boot_dir().join("boot-state")
    }

    pub fn manifest_path(&self, id: u32) -> PathBuf {
        self.db_dir().join(format!("{}.manifest.json", id))
    }

    pub fn init_if_needed(&self) -> Result<(), ControlError> {
        fs::create_dir_all(self.db_dir()).map_err(|e| ControlError::Io(e.to_string()))?;
        fs::create_dir_all(self.boot_dir()).map_err(|e| ControlError::Io(e.to_string()))?;

        // Initialize registry.json if absent
        if !self.registry_path().exists() {
            let reg = GenerationRegistry::baseline();
            self.save_registry(&reg)?;
        }

        // Initialize baseline manifest if absent
        let baseline_manifest_path = self.manifest_path(DEFAULT_BASELINE_GEN_ID);
        if !baseline_manifest_path.exists() {
            let digest = GenerationManifest::compute_digest(
                DEFAULT_BASELINE_GEN_ID,
                None,
                "MahinaOS Baseline System Generation",
                1788739200,
                "Linux 6.6-mahina",
                GenerationStatus::Healthy,
                100,
                true,
                "[]",
                "0000000000000000000000000000000000000000000000000000000000000000",
                "0000000000000000000000000000000000000000000000000000000000000000",
            );

            let baseline_manifest = GenerationManifest {
                id: DEFAULT_BASELINE_GEN_ID,
                parent_id: None,
                description: "MahinaOS Baseline System Generation".to_string(),
                created_at_epoch_secs: 1788739200,
                kernel_version: "Linux 6.6-mahina".to_string(),
                status: GenerationStatus::Healthy,
                btrfs_tree_root_id: 100,
                is_pinned: true,
                commit_digest_sha256: digest,
            };
            self.save_manifest(&baseline_manifest)?;
        }

        // Initialize boot-state if absent
        if !self.boot_state_path().exists() {
            let boot_state = BootState::new(DEFAULT_BASELINE_GEN_ID, DEFAULT_BASELINE_GEN_ID);
            self.save_boot_state(&boot_state)?;
        }

        Ok(())
    }

    pub fn load_registry(&self) -> Result<GenerationRegistry, ControlError> {
        let path = self.registry_path();
        if !path.exists() {
            self.init_if_needed()?;
        }
        let content = fs::read_to_string(&path).map_err(|e| ControlError::Io(e.to_string()))?;
        serde_json::from_str(&content).map_err(|e| ControlError::Serialization(e.to_string()))
    }

    pub fn save_registry(&self, reg: &GenerationRegistry) -> Result<(), ControlError> {
        let json = serde_json::to_string_pretty(reg)
            .map_err(|e| ControlError::Serialization(e.to_string()))?;
        atomic_write_file(&self.registry_path(), json.as_bytes())
    }

    pub fn load_manifest(&self, id: u32) -> Result<GenerationManifest, ControlError> {
        let path = self.manifest_path(id);
        if !path.exists() {
            return Err(ControlError::NotFound(format!("Generation #{} manifest not found", id)));
        }
        let content = fs::read_to_string(&path).map_err(|e| ControlError::Io(e.to_string()))?;
        serde_json::from_str(&content).map_err(|e| ControlError::Serialization(e.to_string()))
    }

    pub fn save_manifest(&self, manifest: &GenerationManifest) -> Result<(), ControlError> {
        let json = serde_json::to_string_pretty(manifest)
            .map_err(|e| ControlError::Serialization(e.to_string()))?;
        atomic_write_file(&self.manifest_path(manifest.id), json.as_bytes())
    }

    pub fn load_boot_state(&self) -> Result<BootState, ControlError> {
        let path = self.boot_state_path();
        if !path.exists() {
            self.init_if_needed()?;
        }
        let content = fs::read_to_string(&path).map_err(|e| ControlError::Io(e.to_string()))?;
        BootState::parse(&content)
    }

    pub fn save_boot_state(&self, boot_state: &BootState) -> Result<(), ControlError> {
        let record = boot_state.to_record();
        atomic_write_file(&self.boot_state_path(), record.as_bytes())
    }

    /// Allocates and increments the next sequential generation ID atomically.
    pub fn allocate_next_id(&self) -> Result<u32, ControlError> {
        let mut reg = self.load_registry()?;
        let allocated = reg.next_generation_id;
        reg.next_generation_id = allocated.saturating_add(1);
        self.save_registry(&reg)?;
        Ok(allocated)
    }

    /// Pin or unpin a generation.
    pub fn set_pinned(&self, id: u32, pinned: bool) -> Result<(), ControlError> {
        let mut reg = self.load_registry()?;
        if pinned {
            if !reg.pinned_generations.contains(&id) {
                reg.pinned_generations.push(id);
            }
        } else {
            // Cannot unpin active current generation or fallback generation if only one exists
            if id == reg.current_generation_id || id == reg.fallback_generation_id {
                return Err(ControlError::PermissionDenied(
                    "Cannot unpin active current or designated fallback generation".to_string(),
                ));
            }
            reg.pinned_generations.retain(|&g| g != id);
        }

        if let Some(s) = reg.generations.iter_mut().find(|g| g.id == id) {
            s.is_pinned = pinned;
        }

        if let Ok(mut manifest) = self.load_manifest(id) {
            manifest.is_pinned = pinned;
            self.save_manifest(&manifest)?;
        }

        self.save_registry(&reg)?;
        Ok(())
    }

    /// Evaluates which generations are eligible for retention pruning.
    /// Strict invariant: Never prune Current, never prune Fallback, never prune Pinned.
    pub fn prunable_generations(&self) -> Result<Vec<u32>, ControlError> {
        let reg = self.load_registry()?;
        if reg.generations.len() <= reg.retention_quota {
            return Ok(Vec::new());
        }

        let mut prunable = Vec::new();
        for g in &reg.generations {
            // Invariant 1: Never prune current
            if g.id == reg.current_generation_id || g.is_current {
                continue;
            }
            // Invariant 2: Never prune fallback
            if g.id == reg.fallback_generation_id || g.is_fallback {
                continue;
            }
            // Invariant 3: Never prune user-pinned
            if g.is_pinned || reg.is_pinned(g.id) {
                continue;
            }
            // Only prune Superseded or Failed generations
            if g.status == GenerationStatus::Superseded || g.status == GenerationStatus::Failed {
                prunable.push(g.id);
            }
        }

        // Sort by ID ascending (oldest first)
        prunable.sort();

        // Calculate how many to prune to satisfy retention quota
        let excess = reg.generations.len().saturating_sub(reg.retention_quota);
        let count_to_prune = prunable.len().min(excess);

        Ok(prunable[..count_to_prune].to_vec())
    }
}

/// Atomically write file by writing to a sibling temporary file, fsyncing, and renaming.
pub fn atomic_write_file(target: &Path, content: &[u8]) -> Result<(), ControlError> {
    let parent = target.parent().ok_or_else(|| ControlError::InvalidParameter {
        param: "target".to_string(),
        reason: format!("Invalid file path parent for {:?}", target),
    })?;

    let file_name = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("tmp_file");
    let tmp_path = parent.join(format!(".{}.tmp", file_name));

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&tmp_path)
        .map_err(|e| ControlError::Io(e.to_string()))?;

    file.write_all(content)
        .map_err(|e| ControlError::Io(e.to_string()))?;
    file.sync_all()
        .map_err(|e| ControlError::Io(e.to_string()))?;
    drop(file);

    fs::rename(&tmp_path, target).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        ControlError::Io(e.to_string())
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_state_parse_and_record() {
        let boot_state = BootState {
            generation_id: 102,
            boot_attempts: 1,
            max_attempts: 3,
            boot_id: "test-boot-uuid".to_string(),
            stage: BootStage::CandidateTrial,
            fallback_generation_id: 101,
            health_status: BootHealthStatus::Pending,
        };

        let record = boot_state.to_record();
        assert!(record.contains("GENERATION_ID=102"));
        assert!(record.contains("BOOT_ATTEMPTS=1"));
        assert!(record.contains("STAGE=CANDIDATE_TRIAL"));
        assert!(record.contains("HEALTH_STATUS=PENDING"));

        let parsed = BootState::parse(&record).expect("BootState parse failed");
        assert_eq!(parsed, boot_state);
    }

    #[test]
    fn test_boot_state_attempt_semantics() {
        let mut state = BootState::candidate_trial(102, 101, "boot-1".to_string());
        assert_eq!(state.boot_attempts, 0);

        // Attempt 1
        let rollback = state.record_attempt();
        assert_eq!(state.boot_attempts, 1);
        assert!(!rollback);

        // Attempt 2
        let rollback = state.record_attempt();
        assert_eq!(state.boot_attempts, 2);
        assert!(!rollback);

        // Attempt 3
        let rollback = state.record_attempt();
        assert_eq!(state.boot_attempts, 3);
        assert!(!rollback);

        // Attempt 4 (Exceeded max_attempts=3 -> automated rollback)
        let rollback = state.record_attempt();
        assert_eq!(state.boot_attempts, 4);
        assert!(rollback);
        assert_eq!(state.stage, BootStage::FailedRollback);
        assert_eq!(state.generation_id, 101);
        assert_eq!(state.health_status, BootHealthStatus::Failed);
    }

    #[test]
    fn test_manifest_canonical_digest_excludes_digest_field() {
        let digest1 = GenerationManifest::compute_digest(
            102,
            Some(101),
            "Candidate Test",
            1788740000,
            "Linux 6.6-mahina",
            GenerationStatus::Candidate,
            102,
            false,
            "pkg1,pkg2",
            "k_hash",
            "initrd_hash",
        );

        let manifest = GenerationManifest {
            id: 102,
            parent_id: Some(101),
            description: "Candidate Test".to_string(),
            created_at_epoch_secs: 1788740000,
            kernel_version: "Linux 6.6-mahina".to_string(),
            status: GenerationStatus::Candidate,
            btrfs_tree_root_id: 102,
            is_pinned: false,
            commit_digest_sha256: digest1.clone(),
        };

        assert!(manifest.verify_digest("pkg1,pkg2", "k_hash", "initrd_hash"));
        // Tampered package manifest must fail
        assert!(!manifest.verify_digest("tampered_pkg", "k_hash", "initrd_hash"));
    }

    #[test]
    fn test_state_store_pruning_invariants() {
        let temp_dir = std::env::temp_dir().join(format!("mahina_state_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);

        let store = StateStore::with_dir(&temp_dir);
        store.init_if_needed().expect("Init state store failed");

        let mut reg = store.load_registry().expect("load registry failed");
        reg.retention_quota = 3;

        // Add generations: 101 (baseline fallback), 102 (superseded), 103 (superseded, pinned), 104 (failed), 105 (current)
        reg.current_generation_id = 105;
        reg.fallback_generation_id = 101;
        reg.pinned_generations = vec![101, 103];

        reg.generations = vec![
            GenerationSummary {
                id: 101,
                parent_id: None,
                description: "Baseline".to_string(),
                created_at_epoch_secs: 1000,
                status: GenerationStatus::Healthy,
                is_pinned: true,
                is_current: false,
                is_fallback: true,
            },
            GenerationSummary {
                id: 102,
                parent_id: Some(101),
                description: "Gen 102".to_string(),
                created_at_epoch_secs: 2000,
                status: GenerationStatus::Superseded,
                is_pinned: false,
                is_current: false,
                is_fallback: false,
            },
            GenerationSummary {
                id: 103,
                parent_id: Some(102),
                description: "Gen 103 (Pinned)".to_string(),
                created_at_epoch_secs: 3000,
                status: GenerationStatus::Superseded,
                is_pinned: true,
                is_current: false,
                is_fallback: false,
            },
            GenerationSummary {
                id: 104,
                parent_id: Some(103),
                description: "Gen 104 (Failed)".to_string(),
                created_at_epoch_secs: 4000,
                status: GenerationStatus::Failed,
                is_pinned: false,
                is_current: false,
                is_fallback: false,
            },
            GenerationSummary {
                id: 105,
                parent_id: Some(103),
                description: "Gen 105 (Current)".to_string(),
                created_at_epoch_secs: 5000,
                status: GenerationStatus::Healthy,
                is_pinned: false,
                is_current: true,
                is_fallback: false,
            },
        ];

        store.save_registry(&reg).expect("save registry failed");

        let prunable = store.prunable_generations().expect("prunable_generations failed");

        // Out of 5 generations with quota 3, 2 should be prunable.
        // 101 is fallback -> Protected
        // 103 is pinned -> Protected
        // 105 is current -> Protected
        // 102 (superseded) and 104 (failed) are prunable
        assert_eq!(prunable, vec![102, 104]);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
