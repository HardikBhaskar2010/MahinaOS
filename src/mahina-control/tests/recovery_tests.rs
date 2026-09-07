/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * recovery_tests.rs — Destructive Recovery Test Suite (Level A).
 *                     Verifies all 6 critical recovery scenarios (Tests A through F):
 *                     A: Clean upgrade and health promotion
 *                     B: Service failure and health attestation refusal
 *                     C: Kernel panic automated reboot and strikeout rollback
 *                     D: Hard hang watchdog timeout recovery
 *                     E: Power loss resilience during atomic writes
 *                     F: Corrupted metadata and digest tampering rejection
 */

use mahina_control::auth::PeerCredentials;
use mahina_control::lifecycle::GenerationLifecycleManager;
use mahina_control::state::{
    atomic_write_file, BootStage, GenerationStatus, DEFAULT_BASELINE_GEN_ID,
};
use std::fs;
use std::path::{Path, PathBuf};

fn setup_test_env(test_name: &str) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let base = std::env::temp_dir().join(format!("mahina_recov_{}_{}", test_name, std::process::id()));
    let state = base.join("state");
    let gen = base.join("generations");
    let root = base.join("rootfs");
    let limine = base.join("boot/limine.conf");

    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&root).unwrap();

    (state, gen, root, limine)
}

fn teardown_test_env(base: &Path) {
    let _ = fs::remove_dir_all(base);
}

#[test]
fn test_scenario_a_successful_upgrade() {
    let (state, gen, root, limine) = setup_test_env("scenario_a");
    let mgr = GenerationLifecycleManager::with_paths(&state, &gen, &limine, true);

    // 1. Current starts at baseline #101
    let initial_reg = mgr.state_store().load_registry().unwrap();
    assert_eq!(initial_reg.current_generation_id, DEFAULT_BASELINE_GEN_ID);

    // 2. Create Candidate #102
    let manifest = mgr
        .create_candidate(&root, "System Upgrade #102", "Linux 6.6.30-mahina", "[]", "k_hash_1", "initrd_hash_1")
        .expect("Failed to create candidate");
    assert_eq!(manifest.id, 102);
    assert_eq!(manifest.status, GenerationStatus::Staged);

    // 3. Activate Candidate #102
    let boot_state = mgr.activate_candidate(102).expect("Failed to activate candidate");
    assert_eq!(boot_state.stage, BootStage::CandidateTrial);
    assert_eq!(boot_state.generation_id, 102);

    // Verify limine.conf staged with candidate as top entry
    let conf = fs::read_to_string(&limine).unwrap();
    assert!(conf.contains("Gen #102 [CANDIDATE TRIAL - TRIES 3]"));
    assert!(conf.contains("mahina.candidate=1 panic=10 panic_on_oops=1"));

    // 4. Successful boot and health attestation by PID 1 (UID 0)
    let peer = PeerCredentials { uid: 0, gid: 0, pid: 1 };
    let promoted = mgr
        .attest_and_mark_healthy(Some(102), &peer)
        .expect("Health attestation failed");
    assert_eq!(promoted.id, 102);
    assert_eq!(promoted.status, GenerationStatus::Healthy);

    // Verify registry updated: 102 is now Current, 101 is Fallback
    let updated_reg = mgr.state_store().load_registry().unwrap();
    assert_eq!(updated_reg.current_generation_id, 102);
    assert_eq!(updated_reg.fallback_generation_id, 101);

    // Verify limine.conf updated: Gen 102 promoted to permanent default entry
    let conf_after = fs::read_to_string(&limine).unwrap();
    assert!(conf_after.contains("Gen #102 [DEFAULT HEALTHY]"));
    assert!(!conf_after.contains("CANDIDATE TRIAL"));

    teardown_test_env(state.parent().unwrap());
}

#[test]
fn test_scenario_b_service_failure_refuses_health() {
    let (state, gen, root, limine) = setup_test_env("scenario_b");
    let mgr = GenerationLifecycleManager::with_paths(&state, &gen, &limine, true);

    // Create & activate candidate
    mgr.create_candidate(&root, "Faulty Candidate #102", "Linux 6.6.30", "[]", "k_hash", "initrd_hash").unwrap();
    mgr.activate_candidate(102).unwrap();

    // Non-root caller attempts health attestation -> MUST BE REJECTED (PermissionDenied)
    let non_root = PeerCredentials { uid: 1000, gid: 1000, pid: 2450 };
    let res = mgr.attest_and_mark_healthy(Some(102), &non_root);
    assert!(res.is_err(), "Health attestation must reject non-root caller");

    // Initiate rollback back to known-good baseline
    let rolled_back = mgr.rollback(None).expect("Rollback failed");
    assert_eq!(rolled_back.id, 101);

    let reg = mgr.state_store().load_registry().unwrap();
    assert_eq!(reg.current_generation_id, 101);

    teardown_test_env(state.parent().unwrap());
}

#[test]
fn test_scenario_c_kernel_panic_boot_attempts_rollback() {
    let (state, gen, root, limine) = setup_test_env("scenario_c");
    let mgr = GenerationLifecycleManager::with_paths(&state, &gen, &limine, true);

    mgr.create_candidate(&root, "Panic Candidate #102", "Linux 6.6.30", "[]", "k_hash", "initrd_hash").unwrap();
    mgr.activate_candidate(102).unwrap();

    // Simulate initramfs boot attempts after kernel panic reboots (panic=10)
    let mut boot_state = mgr.state_store().load_boot_state().unwrap();
    assert_eq!(boot_state.boot_attempts, 0);

    // Boot Attempt 1 (panic reboot)
    let rollback1 = boot_state.record_attempt();
    assert_eq!(boot_state.boot_attempts, 1);
    assert!(!rollback1);

    // Boot Attempt 2 (panic reboot)
    let rollback2 = boot_state.record_attempt();
    assert_eq!(boot_state.boot_attempts, 2);
    assert!(!rollback2);

    // Boot Attempt 3 (panic reboot)
    let rollback3 = boot_state.record_attempt();
    assert_eq!(boot_state.boot_attempts, 3);
    assert!(!rollback3);

    // Boot Attempt 4 -> EXCEEDED max_attempts=3 -> Trigger automated fallback
    let rollback4 = boot_state.record_attempt();
    assert_eq!(boot_state.boot_attempts, 4);
    assert!(rollback4, "Automated rollback must trigger after 3 failed attempts");
    assert_eq!(boot_state.stage, BootStage::FailedRollback);
    assert_eq!(boot_state.generation_id, DEFAULT_BASELINE_GEN_ID);

    // Save and verify state
    mgr.state_store().save_boot_state(&boot_state).unwrap();
    let reloaded = mgr.state_store().load_boot_state().unwrap();
    assert_eq!(reloaded.generation_id, DEFAULT_BASELINE_GEN_ID);

    teardown_test_env(state.parent().unwrap());
}

#[test]
fn test_scenario_d_hard_hang_watchdog_timeout() {
    let (state, gen, root, limine) = setup_test_env("scenario_d");
    let mgr = GenerationLifecycleManager::with_paths(&state, &gen, &limine, true);

    mgr.create_candidate(&root, "Hang Candidate #102", "Linux 6.6.30", "[]", "k_hash", "initrd_hash").unwrap();
    mgr.activate_candidate(102).unwrap();

    // Machine hangs, watchdog resets machine.
    // Early initramfs decrements attempts
    let mut boot_state = mgr.state_store().load_boot_state().unwrap();
    boot_state.record_attempt(); // Attempt 1
    boot_state.record_attempt(); // Attempt 2
    boot_state.record_attempt(); // Attempt 3
    let tripped = boot_state.record_attempt(); // Attempt 4 -> Rollback

    assert!(tripped);
    assert_eq!(boot_state.stage, BootStage::FailedRollback);
    assert_eq!(boot_state.generation_id, DEFAULT_BASELINE_GEN_ID);

    teardown_test_env(state.parent().unwrap());
}

#[test]
fn test_scenario_e_power_loss_atomic_resilience() {
    let (state, gen, _root, limine) = setup_test_env("scenario_e");
    let mgr = GenerationLifecycleManager::with_paths(&state, &gen, &limine, true);

    let store = mgr.state_store();
    store.init_if_needed().unwrap();

    let reg = store.load_registry().unwrap();
    let valid_json = serde_json::to_string_pretty(&reg).unwrap();

    let target_path = store.registry_path();
    atomic_write_file(&target_path, valid_json.as_bytes()).expect("Atomic write failed");

    // Simulate power interruption during writing of another file: temporary file left behind
    let tmp_path = target_path.parent().unwrap().join(".registry.json.tmp");
    fs::write(&tmp_path, b"CORRUPTED_INCOMPLETE_WRITE").unwrap();

    // Verify reading authoritative registry is intact and unaffected by orphaned temp file
    let reloaded = store.load_registry().expect("Authoritative registry must remain intact");
    assert_eq!(reloaded.current_generation_id, DEFAULT_BASELINE_GEN_ID);

    teardown_test_env(state.parent().unwrap());
}

#[test]
fn test_scenario_f_corrupted_metadata_rejection() {
    let (state, gen, root, limine) = setup_test_env("scenario_f");
    let mgr = GenerationLifecycleManager::with_paths(&state, &gen, &limine, true);

    let manifest = mgr
        .create_candidate(&root, "Tampered Candidate #102", "Linux 6.6.30", "[]", "k_hash", "initrd_hash")
        .unwrap();

    // Verify valid digest
    assert!(manifest.verify_digest("[]", "k_hash", "initrd_hash"));

    // Tampered package set must fail digest check
    assert!(!manifest.verify_digest("[\"malicious-package\"]", "k_hash", "initrd_hash"));

    // Tampered kernel hash must fail digest check
    assert!(!manifest.verify_digest("[]", "compromised_kernel", "initrd_hash"));

    teardown_test_env(state.parent().unwrap());
}
