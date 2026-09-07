/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * lifecycle.rs — Generation Lifecycle & Candidate Promotion Manager.
 *               Coordinates transactional candidate creation, TOCTOU-safe candidate activation,
 *               test-driven health attestation, automated and manual rollback, and dynamic
 *               retention pruning.
 */

use crate::auth::PeerCredentials;
use crate::boot::{LimineConfigGenerator, RecoveryCapabilities, DEFAULT_LIMINE_CONF_PATH};
use crate::btrfs::BtrfsEngine;
use crate::error::ControlError;
use crate::health::HealthEvaluator;
use crate::state::{
    BootHealthStatus, BootStage, BootState, GenerationManifest, GenerationStatus,
    GenerationSummary, StateStore,
};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct GenerationLifecycleManager {
    state_store: StateStore,
    btrfs_engine: BtrfsEngine,
    boot_generator: LimineConfigGenerator,
    health_evaluator: HealthEvaluator,
    limine_conf_path: PathBuf,
}

impl Default for GenerationLifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GenerationLifecycleManager {
    pub fn new() -> Self {
        Self {
            state_store: StateStore::new(),
            btrfs_engine: BtrfsEngine::new(),
            boot_generator: LimineConfigGenerator::new(),
            health_evaluator: HealthEvaluator::new(),
            limine_conf_path: PathBuf::from(DEFAULT_LIMINE_CONF_PATH),
        }
    }

    pub fn with_paths(
        state_dir: impl AsRef<Path>,
        generations_root: impl AsRef<Path>,
        limine_conf: impl AsRef<Path>,
        mock_mode: bool,
    ) -> Self {
        let state_store = StateStore::with_dir(&state_dir);
        let btrfs_engine = BtrfsEngine::with_root(&generations_root).with_mock(mock_mode);
        let boot_generator = LimineConfigGenerator::new();
        let health_evaluator = HealthEvaluator::new()
            .with_state_dir(&state_dir)
            .with_mock(mock_mode);

        Self {
            state_store,
            btrfs_engine,
            boot_generator,
            health_evaluator,
            limine_conf_path: limine_conf.as_ref().to_path_buf(),
        }
    }

    pub fn state_store(&self) -> &StateStore {
        &self.state_store
    }

    pub fn recovery_capabilities(&self) -> RecoveryCapabilities {
        RecoveryCapabilities::probe()
    }

    /// Step 1: Create a candidate generation from the current rootfs.
    /// Resulting generation is in `Staged` state.
    pub fn create_candidate(
        &self,
        source_root: &Path,
        description: &str,
        kernel_version: &str,
        package_manifest: &str,
        kernel_sha256: &str,
        initramfs_sha256: &str,
    ) -> Result<GenerationManifest, ControlError> {
        self.state_store.init_if_needed()?;
        self.btrfs_engine.transactional_create_candidate(
            &self.state_store,
            source_root,
            description,
            kernel_version,
            package_manifest,
            kernel_sha256,
            initramfs_sha256,
        )
    }

    /// Step 2: TOCTOU-safe Candidate Activation.
    /// Pre-conditions:
    /// - Generation exists and is in `Staged` status.
    /// - Parent matches current healthy generation.
    ///
    /// Side-effects:
    /// - Atomically sets `boot-state` to CANDIDATE_TRIAL with max 3 attempts.
    /// - Atomically replaces `limine.conf` setting candidate as default entry with `panic=10` and `mahina.candidate=1`.
    /// - Transitions manifest status to `Candidate`.
    pub fn activate_candidate(&self, gen_id: u32) -> Result<BootState, ControlError> {
        self.state_store.init_if_needed()?;

        let mut manifest = self.state_store.load_manifest(gen_id)?;
        if manifest.status != GenerationStatus::Staged && manifest.status != GenerationStatus::Candidate {
            return Err(ControlError::InvalidParameter {
                param: "gen_id".to_string(),
                reason: format!("Generation #{} is in status {:?}, expected Staged", gen_id, manifest.status),
            });
        }

        let reg = self.state_store.load_registry()?;
        if manifest.parent_id != Some(reg.current_generation_id) {
            return Err(ControlError::InvalidParameter {
                param: "parent_id".to_string(),
                reason: format!(
                    "Candidate parent (#{:?}) does not match active healthy generation (#{})",
                    manifest.parent_id, reg.current_generation_id
                ),
            });
        }

        // 1. Atomically write boot-state record
        let boot_id = format!("boot-gen-{}", gen_id);
        let boot_state = BootState::candidate_trial(gen_id, reg.current_generation_id, boot_id);
        self.state_store.save_boot_state(&boot_state)?;

        // 2. Atomically generate and replace limine.conf
        let limine_content = self.boot_generator.generate(&reg, Some(gen_id));
        if let Some(parent) = self.limine_conf_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        self.boot_generator
            .write_atomic(&self.limine_conf_path, &limine_content)?;

        // 3. Update manifest and registry status to Candidate
        manifest.status = GenerationStatus::Candidate;
        self.state_store.save_manifest(&manifest)?;

        let mut updated_reg = reg;
        if let Some(s) = updated_reg.generations.iter_mut().find(|g| g.id == gen_id) {
            s.status = GenerationStatus::Candidate;
        }
        self.state_store.save_registry(&updated_reg)?;

        Ok(boot_state)
    }

    /// Step 3: Test-Driven Health Attestation & Promotion.
    /// Pre-conditions:
    /// - System Attestation: caller must have UID 0 (luna-init PID 1 or system service).
    /// - Level 2 (CORE_HEALTHY) and Level 3 (PLATFORM_HEALTHY) health probes pass.
    ///
    /// Side-effects:
    /// - Atomically sets `boot-state` to NORMAL_BOOT (Healthy).
    /// - Promotes candidate to `Healthy` and `Current`.
    /// - Previous current becomes `Fallback` (Superseded).
    /// - Regenerates `limine.conf` removing candidate trial flags.
    /// - Executes retention policy pruner.
    pub fn attest_and_mark_healthy(
        &self,
        gen_id: Option<u32>,
        caller: &PeerCredentials,
    ) -> Result<GenerationManifest, ControlError> {
        self.state_store.init_if_needed()?;

        // 1. Authenticated System Attestation Check
        if caller.uid != 0 {
            return Err(ControlError::PermissionDenied(
                "Health attestation requires system root credentials (UID 0)".to_string(),
            ));
        }

        // 2. Resolve target generation ID
        let boot_state = self.state_store.load_boot_state()?;
        let target_id = gen_id.unwrap_or(boot_state.generation_id);

        let mut manifest = self.state_store.load_manifest(target_id)?;

        // 3. Run explicit test-driven health evaluation
        let report = self.health_evaluator.evaluate();
        if !report.is_promotable {
            return Err(ControlError::Internal(format!(
                "Health attestation rejected due to failed health checks: {:?}",
                report.details
            )));
        }

        // 4. Update boot-state to NormalBoot (Healthy)
        let updated_boot_state = BootState {
            generation_id: target_id,
            boot_attempts: 0,
            max_attempts: boot_state.max_attempts,
            boot_id: boot_state.boot_id,
            stage: BootStage::NormalBoot,
            fallback_generation_id: boot_state.fallback_generation_id,
            health_status: BootHealthStatus::Healthy,
        };
        self.state_store.save_boot_state(&updated_boot_state)?;

        // 5. Update manifest to Healthy
        manifest.status = GenerationStatus::Healthy;
        self.state_store.save_manifest(&manifest)?;

        // 6. Update GenerationRegistry: promote target to Current, old current to Fallback
        let mut reg = self.state_store.load_registry()?;
        let old_current_id = reg.current_generation_id;

        if old_current_id != target_id {
            reg.fallback_generation_id = old_current_id;
            reg.current_generation_id = target_id;

            for g in &mut reg.generations {
                if g.id == old_current_id {
                    g.is_current = false;
                    g.is_fallback = true;
                    if g.status == GenerationStatus::Healthy {
                        g.status = GenerationStatus::Superseded;
                    }
                } else if g.id == target_id {
                    g.is_current = true;
                    g.is_fallback = false;
                    g.status = GenerationStatus::Healthy;
                }
            }
        } else if let Some(g) = reg.generations.iter_mut().find(|g| g.id == target_id) {
            g.status = GenerationStatus::Healthy;
            g.is_current = true;
        }

        self.state_store.save_registry(&reg)?;

        // 7. Regenerate limine.conf without candidate trial parameters
        let limine_content = self.boot_generator.generate(&reg, None);
        if let Some(parent) = self.limine_conf_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        self.boot_generator
            .write_atomic(&self.limine_conf_path, &limine_content)?;

        // 8. Retention Pruning: enforce "never prune current, fallback, or pinned"
        self.execute_retention_prune()?;

        Ok(manifest)
    }

    /// Step 4: Automated or Manual Rollback.
    /// Immediately reverts boot configuration and boot-state to `target_id` (or designated fallback).
    pub fn rollback(&self, target_id: Option<u32>) -> Result<GenerationSummary, ControlError> {
        self.state_store.init_if_needed()?;

        let mut reg = self.state_store.load_registry()?;
        let fallback_id = target_id.unwrap_or(reg.fallback_generation_id);

        let target_summary = reg
            .get_summary(fallback_id)
            .cloned()
            .ok_or_else(|| ControlError::NotFound(format!("Rollback target generation #{} not found", fallback_id)))?;

        // If current was a candidate, mark it as Failed
        let old_id = reg.current_generation_id;
        if old_id != fallback_id {
            if let Ok(mut old_manifest) = self.state_store.load_manifest(old_id) {
                if old_manifest.status == GenerationStatus::Candidate {
                    old_manifest.status = GenerationStatus::Failed;
                    let _ = self.state_store.save_manifest(&old_manifest);
                }
            }
        }

        // 1. Update boot-state
        let boot_state = BootState {
            generation_id: fallback_id,
            boot_attempts: 0,
            max_attempts: 3,
            boot_id: format!("rollback-to-{}", fallback_id),
            stage: BootStage::FailedRollback,
            fallback_generation_id: fallback_id,
            health_status: BootHealthStatus::Healthy,
        };
        self.state_store.save_boot_state(&boot_state)?;

        // 2. Update registry
        reg.current_generation_id = fallback_id;
        for g in &mut reg.generations {
            if g.id == fallback_id {
                g.is_current = true;
            } else if g.id == old_id {
                g.is_current = false;
                if g.status == GenerationStatus::Candidate {
                    g.status = GenerationStatus::Failed;
                }
            }
        }
        self.state_store.save_registry(&reg)?;

        // 3. Regenerate limine.conf
        let limine_content = self.boot_generator.generate(&reg, None);
        if let Some(parent) = self.limine_conf_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        self.boot_generator
            .write_atomic(&self.limine_conf_path, &limine_content)?;

        Ok(target_summary)
    }

    /// Execute retention sweep. Deletes Btrfs snapshots of prunable generations.
    fn execute_retention_prune(&self) -> Result<Vec<u32>, ControlError> {
        let prunable = self.state_store.prunable_generations()?;
        if prunable.is_empty() {
            return Ok(Vec::new());
        }

        let mut reg = self.state_store.load_registry()?;
        let mut pruned = Vec::new();

        for id in prunable {
            let rootfs = self.btrfs_engine.generations_root().join(id.to_string()).join("rootfs");
            let _ = self.btrfs_engine.delete_snapshot(&rootfs);
            let _ = std::fs::remove_dir_all(self.btrfs_engine.generations_root().join(id.to_string()));

            if let Some(g) = reg.generations.iter_mut().find(|g| g.id == id) {
                g.status = GenerationStatus::Pruned;
            }
            if let Ok(mut m) = self.state_store.load_manifest(id) {
                m.status = GenerationStatus::Pruned;
                let _ = self.state_store.save_manifest(&m);
            }
            pruned.push(id);
        }

        self.state_store.save_registry(&reg)?;
        Ok(pruned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_lifecycle_full_candidate_promotion_flow() {
        let temp_base = std::env::temp_dir().join(format!("mahina_lifecycle_test_{}", std::process::id()));
        let temp_state = temp_base.join("state");
        let temp_gen = temp_base.join("generations");
        let temp_root = temp_base.join("rootfs");
        let limine_conf = temp_base.join("boot/limine.conf");

        let _ = fs::remove_dir_all(&temp_base);
        fs::create_dir_all(&temp_root).unwrap();

        let mgr = GenerationLifecycleManager::with_paths(&temp_state, &temp_gen, &limine_conf, true);

        // 1. Create candidate
        let manifest = mgr
            .create_candidate(&temp_root, "Upgrade 102", "Linux 6.6.30", "[]", "k_hash", "initrd_hash")
            .expect("create_candidate failed");
        assert_eq!(manifest.id, 102);
        assert_eq!(manifest.status, GenerationStatus::Staged);

        // 2. Activate candidate
        let boot_state = mgr.activate_candidate(102).expect("activate_candidate failed");
        assert_eq!(boot_state.stage, BootStage::CandidateTrial);
        assert_eq!(boot_state.generation_id, 102);

        // Verify limine.conf contains candidate trial
        let conf = fs::read_to_string(&limine_conf).unwrap();
        assert!(conf.contains("Gen #102 [CANDIDATE TRIAL - TRIES 3]"));

        // 3. Health attestation from PID 1 (UID 0)
        let root_peer = PeerCredentials { uid: 0, gid: 0, pid: 1 };
        let promoted = mgr
            .attest_and_mark_healthy(Some(102), &root_peer)
            .expect("attest_and_mark_healthy failed");
        assert_eq!(promoted.id, 102);
        assert_eq!(promoted.status, GenerationStatus::Healthy);

        // Verify registry updated: 102 is current, 101 is fallback
        let reg = mgr.state_store().load_registry().unwrap();
        assert_eq!(reg.current_generation_id, 102);
        assert_eq!(reg.fallback_generation_id, 101);

        // Verify limine.conf updated: Gen 102 is now default healthy entry
        let conf2 = fs::read_to_string(&limine_conf).unwrap();
        assert!(conf2.contains("Gen #102 [DEFAULT HEALTHY]"));
        assert!(!conf2.contains("CANDIDATE TRIAL"));

        let _ = fs::remove_dir_all(&temp_base);
    }

    #[test]
    fn test_lifecycle_rollback_flow() {
        let temp_base = std::env::temp_dir().join(format!("mahina_lifecycle_rb_test_{}", std::process::id()));
        let temp_state = temp_base.join("state");
        let temp_gen = temp_base.join("generations");
        let temp_root = temp_base.join("rootfs");
        let limine_conf = temp_base.join("boot/limine.conf");

        let _ = fs::remove_dir_all(&temp_base);
        fs::create_dir_all(&temp_root).unwrap();

        let mgr = GenerationLifecycleManager::with_paths(&temp_state, &temp_gen, &limine_conf, true);

        // Create & activate candidate 102
        mgr.create_candidate(&temp_root, "Upgrade 102", "Linux 6.6.30", "[]", "k_hash", "initrd_hash").unwrap();
        mgr.activate_candidate(102).unwrap();

        // Rollback triggered
        let rolled_back = mgr.rollback(None).expect("rollback failed");
        assert_eq!(rolled_back.id, 101);

        let reg = mgr.state_store().load_registry().unwrap();
        assert_eq!(reg.current_generation_id, 101);

        let conf = fs::read_to_string(&limine_conf).unwrap();
        assert!(conf.contains("Gen #101 [DEFAULT HEALTHY]"));

        let _ = fs::remove_dir_all(&temp_base);
    }
}
