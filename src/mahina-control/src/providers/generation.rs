/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * generation.rs — Generation Provider backed by the isolated StateStore (/system/mahina-state).
 *                 Single-writer provider orchestrating generation listings, candidate creation,
 *                 TOCTOU-safe activation, health attestation, rollback, and pin management.
 */

use crate::auth::PeerCredentials;
use crate::boot::RecoveryCapabilities;
use crate::error::ControlError;
use crate::lifecycle::GenerationLifecycleManager;
use crate::protocol::GenerationEntry;
use crate::state::{GenerationStatus, StateStore, DEFAULT_BASELINE_GEN_ID};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct GenerationProvider {
    lifecycle_mgr: GenerationLifecycleManager,
}

impl GenerationProvider {
    pub fn new() -> Self {
        Self {
            lifecycle_mgr: GenerationLifecycleManager::new(),
        }
    }

    pub fn with_lifecycle(mgr: GenerationLifecycleManager) -> Self {
        Self { lifecycle_mgr: mgr }
    }

    pub fn with_state_dir(state_dir: impl AsRef<Path>) -> Self {
        let mgr = GenerationLifecycleManager::with_paths(
            state_dir,
            "/generations",
            "/boot/efi/limine.conf",
            false,
        );
        Self { lifecycle_mgr: mgr }
    }

    pub fn state_store(&self) -> &StateStore {
        self.lifecycle_mgr.state_store()
    }

    pub fn recovery_capabilities(&self) -> RecoveryCapabilities {
        self.lifecycle_mgr.recovery_capabilities()
    }

    pub fn get_current(&self) -> Result<GenerationEntry, ControlError> {
        if let Ok(reg) = self.state_store().load_registry() {
            let cur_id = reg.current_generation_id;
            if let Ok(manifest) = self.state_store().load_manifest(cur_id) {
                return Ok(GenerationEntry {
                    id: manifest.id,
                    description: manifest.description,
                    created_at_epoch_secs: manifest.created_at_epoch_secs,
                    is_current: true,
                    is_healthy: manifest.status == GenerationStatus::Healthy,
                    kernel_version: manifest.kernel_version,
                });
            }
            if let Some(summary) = reg.get_summary(cur_id) {
                return Ok(GenerationEntry {
                    id: summary.id,
                    description: summary.description.clone(),
                    created_at_epoch_secs: summary.created_at_epoch_secs,
                    is_current: true,
                    is_healthy: summary.status == GenerationStatus::Healthy,
                    kernel_version: "Linux 6.6-mahina".to_string(),
                });
            }
        }

        // Fallback baseline generation
        Ok(GenerationEntry {
            id: DEFAULT_BASELINE_GEN_ID,
            description: "MahinaOS Baseline System Generation".to_string(),
            created_at_epoch_secs: 1788739200,
            is_current: true,
            is_healthy: true,
            kernel_version: "Linux 6.6-mahina".to_string(),
        })
    }

    pub fn list(&self) -> Result<Vec<GenerationEntry>, ControlError> {
        if let Ok(reg) = self.state_store().load_registry() {
            let mut entries = Vec::new();
            for s in reg.generations {
                let kernel_ver = self
                    .state_store()
                    .load_manifest(s.id)
                    .map(|m| m.kernel_version)
                    .unwrap_or_else(|_| "Linux 6.6-mahina".to_string());

                entries.push(GenerationEntry {
                    id: s.id,
                    description: s.description,
                    created_at_epoch_secs: s.created_at_epoch_secs,
                    is_current: s.id == reg.current_generation_id,
                    is_healthy: s.status == GenerationStatus::Healthy,
                    kernel_version: kernel_ver,
                });
            }

            if !entries.is_empty() {
                entries.sort_by_key(|e| e.id);
                return Ok(entries);
            }
        }

        // Fallback baseline entry
        Ok(vec![GenerationEntry {
            id: DEFAULT_BASELINE_GEN_ID,
            description: "MahinaOS Baseline System Generation".to_string(),
            created_at_epoch_secs: 1788739200,
            is_current: true,
            is_healthy: true,
            kernel_version: "Linux 6.6-mahina".to_string(),
        }])
    }

    pub fn create(&self, description: &str) -> Result<GenerationEntry, ControlError> {
        let manifest = self.lifecycle_mgr.create_candidate(
            Path::new("/"),
            description,
            "Linux 6.6-mahina",
            "[]",
            "k_default",
            "initrd_default",
        )?;

        Ok(GenerationEntry {
            id: manifest.id,
            description: manifest.description,
            created_at_epoch_secs: manifest.created_at_epoch_secs,
            is_current: false,
            is_healthy: false,
            kernel_version: manifest.kernel_version,
        })
    }

    pub fn activate(&self, id: u32) -> Result<String, ControlError> {
        let boot_state = self.lifecycle_mgr.activate_candidate(id)?;
        Ok(format!(
            "Candidate generation #{} activated for trial boot (max attempts: {})",
            boot_state.generation_id, boot_state.max_attempts
        ))
    }

    pub fn mark_healthy(
        &self,
        id: Option<u32>,
        caller: &PeerCredentials,
    ) -> Result<GenerationEntry, ControlError> {
        let manifest = self.lifecycle_mgr.attest_and_mark_healthy(id, caller)?;
        Ok(GenerationEntry {
            id: manifest.id,
            description: manifest.description,
            created_at_epoch_secs: manifest.created_at_epoch_secs,
            is_current: true,
            is_healthy: true,
            kernel_version: manifest.kernel_version,
        })
    }

    pub fn rollback(&self, target_id: Option<u32>) -> Result<GenerationEntry, ControlError> {
        let summary = self.lifecycle_mgr.rollback(target_id)?;
        Ok(GenerationEntry {
            id: summary.id,
            description: summary.description,
            created_at_epoch_secs: summary.created_at_epoch_secs,
            is_current: true,
            is_healthy: true,
            kernel_version: "Linux 6.6-mahina".to_string(),
        })
    }

    pub fn pin(&self, id: u32, pinned: bool) -> Result<(), ControlError> {
        self.state_store().set_pinned(id, pinned)
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
    use std::fs;

    #[test]
    fn test_generation_provider_backed_by_state_store() {
        let temp_dir = std::env::temp_dir().join(format!("mahina_gen_prov_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);

        let provider = GenerationProvider::with_state_dir(&temp_dir);
        let current = provider.get_current().expect("get_current failed");
        assert_eq!(current.id, DEFAULT_BASELINE_GEN_ID);
        assert!(current.is_current);
        assert!(current.is_healthy);

        let list = provider.list().expect("list failed");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, DEFAULT_BASELINE_GEN_ID);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
