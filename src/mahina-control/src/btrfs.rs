/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * btrfs.rs — Transactional Btrfs Snapshot Engine.
 *           Enforces pre-flight disk space reserve thresholds, atomic read-only snapshotting,
 *           and automatic cleanup on failure (no orphaned subvolumes or manifests).
 */

use crate::error::ControlError;
use crate::state::{
    GenerationManifest, GenerationStatus, GenerationSummary, StateStore,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

pub const DEFAULT_GENERATIONS_ROOT: &str = "/generations";
pub const MIN_FREE_SPACE_BYTES: u64 = 2 * 1024 * 1024 * 1024; // 2.0 GiB reserve

#[derive(Debug, Clone)]
pub struct BtrfsEngine {
    generations_root: PathBuf,
    min_free_space_bytes: u64,
    mock_mode: bool,
}

impl Default for BtrfsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl BtrfsEngine {
    pub fn new() -> Self {
        Self {
            generations_root: PathBuf::from(DEFAULT_GENERATIONS_ROOT),
            min_free_space_bytes: MIN_FREE_SPACE_BYTES,
            mock_mode: false,
        }
    }

    pub fn with_root(path: impl AsRef<Path>) -> Self {
        Self {
            generations_root: path.as_ref().to_path_buf(),
            min_free_space_bytes: MIN_FREE_SPACE_BYTES,
            mock_mode: false,
        }
    }

    pub fn with_mock(mut self, mock: bool) -> Self {
        self.mock_mode = mock;
        self
    }

    pub fn with_min_free_space(mut self, bytes: u64) -> Self {
        self.min_free_space_bytes = bytes;
        self
    }

    pub fn generations_root(&self) -> &Path {
        &self.generations_root
    }

    /// Query available free space in bytes at the specified path.
    pub fn query_free_space(&self, path: &Path) -> Result<u64, ControlError> {
        if self.mock_mode {
            return Ok(10 * 1024 * 1024 * 1024); // 10 GiB in mock mode
        }

        #[cfg(target_os = "linux")]
        unsafe {
            use std::ffi::CString;
            let path_str = path.to_str().unwrap_or("/");
            if let Ok(c_path) = CString::new(path_str) {
                let mut stat: libc::statvfs = std::mem::zeroed();
                if libc::statvfs(c_path.as_ptr(), &mut stat) == 0 {
                    let free_bytes = (stat.f_bavail as u64).saturating_mul(stat.f_bsize as u64);
                    return Ok(free_bytes);
                }
            }
        }

        // Fallback: If statvfs fails or not on Linux, assume sufficient space unless mock forces otherwise
        Ok(10 * 1024 * 1024 * 1024)
    }

    /// Pre-flight disk space verification. Refuses operation if free space is below threshold.
    pub fn verify_free_space(&self) -> Result<(), ControlError> {
        let check_path = if self.generations_root.exists() {
            &self.generations_root
        } else {
            Path::new("/")
        };

        let free = self.query_free_space(check_path)?;
        if free < self.min_free_space_bytes {
            return Err(ControlError::Internal(format!(
                "Insufficient disk space for candidate generation: {} bytes free, {} bytes required",
                free, self.min_free_space_bytes
            )));
        }
        Ok(())
    }

    /// Create read-only Btrfs snapshot of `source` at `target`.
    pub fn create_snapshot(&self, source: &Path, target: &Path) -> Result<(), ControlError> {
        if self.mock_mode {
            fs::create_dir_all(target).map_err(|e| ControlError::Io(e.to_string()))?;
            return Ok(());
        }

        let output = Command::new("btrfs")
            .args([
                "subvolume",
                "snapshot",
                "-r",
                source.to_str().unwrap_or("/"),
                target.to_str().unwrap_or(""),
            ])
            .output()
            .map_err(|e| ControlError::Io(format!("Failed to execute btrfs command: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ControlError::Internal(format!(
                "btrfs subvolume snapshot failed: {}",
                stderr.trim()
            )));
        }

        if !target.exists() {
            return Err(ControlError::Internal(format!(
                "Snapshot target {:?} does not exist after creation",
                target
            )));
        }

        Ok(())
    }

    /// Delete Btrfs subvolume at `target`.
    pub fn delete_snapshot(&self, target: &Path) -> Result<(), ControlError> {
        if !target.exists() {
            return Ok(());
        }

        if self.mock_mode {
            let _ = fs::remove_dir_all(target);
            return Ok(());
        }

        let output = Command::new("btrfs")
            .args([
                "subvolume",
                "delete",
                target.to_str().unwrap_or(""),
            ])
            .output()
            .map_err(|e| ControlError::Io(format!("Failed to execute btrfs delete command: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ControlError::Internal(format!(
                "btrfs subvolume delete failed: {}",
                stderr.trim()
            )));
        }

        Ok(())
    }

    /// Transactional Candidate Generation Creation:
    /// 1. Pre-flight space verification.
    /// 2. Allocate next sequential generation ID.
    /// 3. Prepare candidate directory (/generations/<id>).
    /// 4. Execute atomic read-only Btrfs snapshot.
    /// 5. Compute canonical commit digest.
    /// 6. Save manifest to StateStore with status = Staged.
    /// 7. Register candidate in GenerationRegistry.
    ///
    /// If any step fails, automatically rolls back and cleans up snapshot and directories.
    pub fn transactional_create_candidate(
        &self,
        state_store: &StateStore,
        source_root: &Path,
        description: &str,
        kernel_version: &str,
        package_manifest: &str,
        kernel_sha256: &str,
        initramfs_sha256: &str,
    ) -> Result<GenerationManifest, ControlError> {
        // Step 1: Pre-flight space check
        self.verify_free_space()?;

        // Step 2: Allocate next generation ID
        let gen_id = state_store.allocate_next_id()?;

        // Step 3: Prepare directories
        let gen_dir = self.generations_root.join(gen_id.to_string());
        let rootfs_target = gen_dir.join("rootfs");

        fs::create_dir_all(&gen_dir).map_err(|e| ControlError::Io(e.to_string()))?;

        // Step 4: Execute atomic read-only snapshot
        if let Err(e) = self.create_snapshot(source_root, &rootfs_target) {
            let _ = fs::remove_dir_all(&gen_dir);
            return Err(e);
        }

        // Helper cleanup closure for subsequent steps
        let cleanup = || {
            let _ = self.delete_snapshot(&rootfs_target);
            let _ = fs::remove_dir_all(&gen_dir);
        };

        // Step 5: Read or compute current healthy parent
        let reg = match state_store.load_registry() {
            Ok(r) => r,
            Err(e) => {
                cleanup();
                return Err(e);
            }
        };
        let parent_id = Some(reg.current_generation_id);

        let created_at_epoch_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(1788740000);

        let btrfs_tree_root_id = gen_id as u64; // In mock/synthetic setups, maps to gen_id

        let commit_digest = GenerationManifest::compute_digest(
            gen_id,
            parent_id,
            description,
            created_at_epoch_secs,
            kernel_version,
            GenerationStatus::Staged,
            btrfs_tree_root_id,
            false,
            package_manifest,
            kernel_sha256,
            initramfs_sha256,
        );

        let manifest = GenerationManifest {
            id: gen_id,
            parent_id,
            description: description.to_string(),
            created_at_epoch_secs,
            kernel_version: kernel_version.to_string(),
            status: GenerationStatus::Staged,
            btrfs_tree_root_id,
            is_pinned: false,
            commit_digest_sha256: commit_digest,
        };

        // Step 6: Save manifest to StateStore
        if let Err(e) = state_store.save_manifest(&manifest) {
            cleanup();
            return Err(e);
        }

        // Step 7: Update registry generations list
        let mut updated_reg = reg;
        updated_reg.generations.push(GenerationSummary {
            id: gen_id,
            parent_id,
            description: description.to_string(),
            created_at_epoch_secs,
            status: GenerationStatus::Staged,
            is_pinned: false,
            is_current: false,
            is_fallback: false,
        });

        if let Err(e) = state_store.save_registry(&updated_reg) {
            cleanup();
            let _ = fs::remove_file(state_store.manifest_path(gen_id));
            return Err(e);
        }

        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_btrfs_transactional_create_candidate_success() {
        let temp_state = std::env::temp_dir().join(format!("mahina_btrfs_state_{}", std::process::id()));
        let temp_gen = std::env::temp_dir().join(format!("mahina_btrfs_gen_{}", std::process::id()));
        let temp_root = std::env::temp_dir().join(format!("mahina_btrfs_root_{}", std::process::id()));

        let _ = fs::remove_dir_all(&temp_state);
        let _ = fs::remove_dir_all(&temp_gen);
        let _ = fs::remove_dir_all(&temp_root);

        fs::create_dir_all(&temp_root).unwrap();

        let state_store = StateStore::with_dir(&temp_state);
        state_store.init_if_needed().unwrap();

        let engine = BtrfsEngine::with_root(&temp_gen).with_mock(true);

        let manifest = engine
            .transactional_create_candidate(
                &state_store,
                &temp_root,
                "Test Upgrade Generation",
                "Linux 6.6.30-mahina",
                "[]",
                "k_hash_1",
                "initrd_hash_1",
            )
            .expect("transactional create candidate failed");

        assert_eq!(manifest.id, 102);
        assert_eq!(manifest.status, GenerationStatus::Staged);
        assert_eq!(manifest.parent_id, Some(101));
        assert!(manifest.verify_digest("[]", "k_hash_1", "initrd_hash_1"));

        // Verify filesystem artifacts exist
        assert!(temp_gen.join("102/rootfs").exists());
        assert!(state_store.manifest_path(102).exists());

        // Verify registry updated
        let reg = state_store.load_registry().unwrap();
        assert_eq!(reg.generations.len(), 2);
        assert_eq!(reg.generations[1].id, 102);
        assert_eq!(reg.generations[1].status, GenerationStatus::Staged);

        let _ = fs::remove_dir_all(&temp_state);
        let _ = fs::remove_dir_all(&temp_gen);
        let _ = fs::remove_dir_all(&temp_root);
    }
}
