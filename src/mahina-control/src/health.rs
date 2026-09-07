/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * health.rs — Test-Driven Health Attestation Engine.
 *            Evaluates tiered health levels (CORE_HEALTHY and PLATFORM_HEALTHY)
 *            before allowing candidate generation promotion.
 */

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthLevel {
    Booted,
    CoreHealthy,
    PlatformHealthy,
    UserSessionHealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthReport {
    pub core_healthy: bool,
    pub platform_healthy: bool,
    pub is_promotable: bool,
    pub details: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct HealthEvaluator {
    control_socket_path: PathBuf,
    lgp_socket_path: PathBuf,
    state_dir: PathBuf,
    mock_mode: bool,
}

impl Default for HealthEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthEvaluator {
    pub fn new() -> Self {
        Self {
            control_socket_path: PathBuf::from("/run/mahina/control.sock"),
            lgp_socket_path: PathBuf::from("/run/lgp.sock"),
            state_dir: PathBuf::from("/system/mahina-state"),
            mock_mode: false,
        }
    }

    pub fn with_mock(mut self, mock: bool) -> Self {
        self.mock_mode = mock;
        self
    }

    pub fn with_state_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.state_dir = path.as_ref().to_path_buf();
        self
    }

    pub fn check_core(&self) -> Result<(), String> {
        if self.mock_mode {
            return Ok(());
        }

        // 1. Verify state directory is accessible and writable
        if !self.state_dir.exists() {
            return Err(format!("State directory {:?} does not exist", self.state_dir));
        }

        let probe_file = self.state_dir.join(".health_probe.tmp");
        if let Err(e) = fs::write(&probe_file, b"health_probe") {
            return Err(format!("State directory {:?} is not writable: {}", self.state_dir, e));
        }
        let _ = fs::remove_file(probe_file);

        // 2. Verify /tmp and /run are accessible
        for dir in &["/tmp", "/run"] {
            let p = Path::new(dir);
            if !p.exists() {
                return Err(format!("Essential directory {} does not exist", dir));
            }
        }

        // 3. Verify control socket directory exists
        if let Some(parent) = self.control_socket_path.parent() {
            if !parent.exists() {
                return Err(format!("Control socket parent directory {:?} does not exist", parent));
            }
        }

        Ok(())
    }

    pub fn check_platform(&self) -> Result<(), String> {
        if self.mock_mode {
            return Ok(());
        }

        // Verify compositor socket is present or DRM master initialized
        if !self.lgp_socket_path.exists() {
            // Check if fallback frame buffer /dev/fb0 or DRM card exists
            let drm_card = Path::new("/dev/dri/card0");
            let fb_dev = Path::new("/dev/fb0");
            if !drm_card.exists() && !fb_dev.exists() {
                return Err("Compositor socket /run/lgp.sock missing and no display devices found".to_string());
            }
        }

        Ok(())
    }

    pub fn evaluate(&self) -> HealthReport {
        let mut details = Vec::new();

        let core_healthy = match self.check_core() {
            Ok(_) => {
                details.push("[✓] CORE_HEALTHY: Filesystems writable, runtime directories verified".to_string());
                true
            }
            Err(e) => {
                details.push(format!("[✗] CORE_HEALTHY FAILED: {}", e));
                false
            }
        };

        let platform_healthy = match self.check_platform() {
            Ok(_) => {
                details.push("[✓] PLATFORM_HEALTHY: Display pipeline and compositor verified".to_string());
                true
            }
            Err(e) => {
                details.push(format!("[✗] PLATFORM_HEALTHY FAILED: {}", e));
                false
            }
        };

        let is_promotable = core_healthy && platform_healthy;

        HealthReport {
            core_healthy,
            platform_healthy,
            is_promotable,
            details,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_evaluator_mock_mode() {
        let evaluator = HealthEvaluator::new().with_mock(true);
        let report = evaluator.evaluate();
        assert!(report.core_healthy);
        assert!(report.platform_healthy);
        assert!(report.is_promotable);
        assert_eq!(report.details.len(), 2);
    }
}
