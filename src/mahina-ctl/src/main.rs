use mahina_control::daemon::DEFAULT_SOCKET_PATH;
use mahina_control::ControlClient;
use std::env;
use std::path::PathBuf;
use std::process;

fn print_help() {
    println!(
        "mahina-ctl — Administrative Interface to MahinaOS System Control Plane\n\n\
         USAGE:\n  \
         mahina-ctl [OPTIONS] <COMMAND> [SUBCOMMAND] [ARGS...]\n\n\
         GLOBAL OPTIONS:\n  \
         -s, --socket <PATH>     Control Plane Unix socket (default: {})\n  \
         -j, --json              Output machine-readable JSON\n  \
         -t, --token <TOKEN>     Capability authorization token for elevated tasks\n  \
         -h, --help              Print help information\n  \
         -v, --version           Print version information\n\n\
         SYSTEM COMMANDS:\n  \
         system state            Query holistic system telemetry and state\n  \
         system reboot           Initiate authorized system reboot\n  \
         system shutdown         Initiate authorized system shutdown\n\n\
         SERVICE COMMANDS:\n  \
         service list            List all supervised services and runtime states\n  \
         service status [NAME]   Show detailed status of a service or all services\n  \
         service start <NAME>    Start a service\n  \
         service stop <NAME>     Stop a service\n  \
         service restart <NAME>  Restart a service\n  \
         service reload <NAME>   Reload a service configuration (SIGHUP)\n\n\
         USER COMMANDS:\n  \
         user list               List local user accounts and group memberships\n\n\
         PACKAGE COMMANDS:\n  \
         package list            List installed software packages\n  \
         package install <TARGET> Install a package (.lpkg or repository package)\n  \
         package remove <NAME>   Remove an installed package\n\n\
         STORAGE COMMANDS:\n  \
         storage list            List mounted filesystems and block storage devices\n  \
         storage mount <SRC> <TGT> Mount a filesystem\n\n\
         GENERATION COMMANDS:\n  \
         generation current      Show currently active system generation\n  \
         generation list         List all system generations and rollback targets\n  \
         generation create <DESC> Create a new candidate generation snapshot\n  \
         generation activate <ID> Schedule candidate generation for trial boot\n  \
         generation mark-healthy Assert health and promote generation to default\n  \
         generation rollback [ID] Roll back to fallback or designated generation\n  \
         generation pin <ID>     Protect generation from automated retention pruning\n  \
         generation unpin <ID>   Remove retention pin from generation\n  \
         generation capabilities Display boot watchdog and recovery capabilities\n",
        DEFAULT_SOCKET_PATH
    );
}

fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * 1024;
    const GIB: u64 = 1024 * 1024 * 1024;
    const TIB: u64 = 1024 * 1024 * 1024 * 1024;

    if bytes >= TIB {
        format!("{:.2} TiB", bytes as f64 / TIB as f64)
    } else if bytes >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.2} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.2} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, seconds)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_help();
        process::exit(1);
    }

    let mut socket_path = PathBuf::from(DEFAULT_SOCKET_PATH);
    let mut json_mode = false;
    let mut token: Option<String> = None;
    let mut positional: Vec<String> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "-v" | "--version" => {
                println!("mahina-ctl {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "-j" | "--json" => {
                json_mode = true;
            }
            "-s" | "--socket" => {
                if i + 1 < args.len() {
                    socket_path = PathBuf::from(&args[i + 1]);
                    i += 1;
                } else {
                    eprintln!("Error: --socket requires a path argument");
                    process::exit(1);
                }
            }
            "-t" | "--token" => {
                if i + 1 < args.len() {
                    token = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    eprintln!("Error: --token requires a token argument");
                    process::exit(1);
                }
            }
            arg => {
                positional.push(arg.to_string());
            }
        }
        i += 1;
    }

    if positional.is_empty() {
        print_help();
        process::exit(1);
    }

    let mut client = match ControlClient::connect(&socket_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error connecting to Mahina Control Plane at {:?}:", socket_path);
            eprintln!("  {}", e);
            eprintln!("Hint: Ensure mahina-control-d is running and you have proper permissions.");
            process::exit(1);
        }
    };

    let cmd = positional.first().map(|s| s.as_str());
    let sub = positional.get(1).map(|s| s.as_str());
    let target = positional.get(2).map(|s| s.as_str());

    match (cmd, sub) {
        (Some("system"), Some("state")) => {
            match client.system_get_state() {
                Ok(state) => {
                    if json_mode {
                        println!("{}", serde_json::to_string_pretty(&state).unwrap_or_default());
                    } else {
                        println!("==================================================");
                        println!("          MahinaOS System Telemetry");
                        println!("==================================================");
                        println!("  Hostname:           {}", state.hostname);
                        println!("  Active Generation:  #{}", state.active_generation);
                        println!("  Kernel Version:     {}", state.kernel_version);
                        println!("  CPUs:               {}", state.cpu_count);
                        println!("  Uptime:             {} ({}s)", format_uptime(state.uptime_secs), state.uptime_secs);
                        let mem_pct = if state.mem_total_mb > 0 {
                            (state.mem_used_mb as f64 / state.mem_total_mb as f64) * 100.0
                        } else {
                            0.0
                        };
                        println!(
                            "  Memory:             {} MiB / {} MiB ({:.1}% used)",
                            state.mem_used_mb,
                            state.mem_total_mb,
                            mem_pct
                        );
                        println!("==================================================");
                    }
                }
                Err(e) => {
                    eprintln!("Error: failed to query system state: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("system"), Some("reboot")) => {
            match client.system_reboot(token) {
                Ok(()) => {
                    println!("Reboot request accepted by Mahina Control Plane.");
                }
                Err(e) => {
                    eprintln!("Error: system reboot rejected: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("system"), Some("shutdown")) => {
            match client.system_shutdown(token) {
                Ok(()) => {
                    println!("Shutdown request accepted by Mahina Control Plane.");
                }
                Err(e) => {
                    eprintln!("Error: system shutdown rejected: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("service") | Some("services"), Some("list")) => {
            match client.services_list() {
                Ok(services) => {
                    if json_mode {
                        println!("{}", serde_json::to_string_pretty(&services).unwrap_or_default());
                    } else if services.is_empty() {
                        println!("No services currently registered in supervisor.");
                    } else {
                        println!("{:<24} {:<16} {:<8}", "SERVICE", "STATE", "PID");
                        println!("{:-<24} {:-<16} {:-<8}", "", "", "");
                        for s in services {
                            let pid_str = if s.pid > 0 { s.pid.to_string() } else { "-".to_string() };
                            println!("{:<24} {:<16} {:<8}", s.name, s.state, pid_str);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: failed to list services: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("service") | Some("services"), Some("status")) => {
            match client.services_status(target) {
                Ok(status) => {
                    if json_mode {
                        println!("{}", serde_json::to_string_pretty(&status).unwrap_or_default());
                    } else {
                        println!("==================================================");
                        println!("          Mahina Service Status: {}", status.name);
                        println!("==================================================");
                        println!("  State:          {}", status.state);
                        let pid_str = if status.pid > 0 { status.pid.to_string() } else { "-".to_string() };
                        println!("  PID:            {}", pid_str);
                        println!("  Restarts:       {}", status.restart_count);
                        println!("==================================================");
                    }
                }
                Err(e) => {
                    eprintln!("Error: failed to query service status: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("service") | Some("services"), Some("start")) => {
            let name = match target {
                Some(n) => n,
                None => {
                    eprintln!("Error: service start requires a service name");
                    process::exit(1);
                }
            };
            match client.services_start(name, token) {
                Ok(msg) => println!("{}", msg),
                Err(e) => {
                    eprintln!("Error: service start failed: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("service") | Some("services"), Some("stop")) => {
            let name = match target {
                Some(n) => n,
                None => {
                    eprintln!("Error: service stop requires a service name");
                    process::exit(1);
                }
            };
            match client.services_stop(name, token) {
                Ok(msg) => println!("{}", msg),
                Err(e) => {
                    eprintln!("Error: service stop failed: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("service") | Some("services"), Some("restart")) => {
            let name = match target {
                Some(n) => n,
                None => {
                    eprintln!("Error: service restart requires a service name");
                    process::exit(1);
                }
            };
            match client.services_restart(name, token) {
                Ok(msg) => println!("{}", msg),
                Err(e) => {
                    eprintln!("Error: service restart failed: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("service") | Some("services"), Some("reload")) => {
            let name = match target {
                Some(n) => n,
                None => {
                    eprintln!("Error: service reload requires a service name");
                    process::exit(1);
                }
            };
            match client.services_reload(name, token) {
                Ok(msg) => println!("{}", msg),
                Err(e) => {
                    eprintln!("Error: service reload failed: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("user") | Some("users"), Some("list")) => {
            match client.users_list() {
                Ok(users) => {
                    if json_mode {
                        println!("{}", serde_json::to_string_pretty(&users).unwrap_or_default());
                    } else if users.is_empty() {
                        println!("No local user accounts found.");
                    } else {
                        println!("{:<16} {:<6} {:<6} {:<24} {:<16} {:<20}", "USER", "UID", "GID", "HOME", "SHELL", "GROUPS");
                        println!("{:-<16} {:-<6} {:-<6} {:-<24} {:-<16} {:-<20}", "", "", "", "", "", "");
                        for u in users {
                            let grps = u.groups.join(", ");
                            println!("{:<16} {:<6} {:<6} {:<24} {:<16} {:<20}", u.username, u.uid, u.gid, u.home_dir, u.shell, grps);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: failed to list users: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("package") | Some("packages"), Some("list")) => {
            match client.packages_list() {
                Ok(pkgs) => {
                    if json_mode {
                        println!("{}", serde_json::to_string_pretty(&pkgs).unwrap_or_default());
                    } else if pkgs.is_empty() {
                        println!("No packages currently installed.");
                    } else {
                        println!("{:<20} {:<12} {:<8} {:<36}", "PACKAGE", "VERSION", "FILES", "DESCRIPTION");
                        println!("{:-<20} {:-<12} {:-<8} {:-<36}", "", "", "", "");
                        for p in pkgs {
                            println!("{:<20} {:<12} {:<8} {:<36}", p.name, p.version, p.file_count, p.description);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: failed to list packages: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("package") | Some("packages"), Some("install")) => {
            let target = match target {
                Some(t) => t,
                None => {
                    eprintln!("Error: package install requires a package target or file");
                    process::exit(1);
                }
            };
            match client.packages_install(target, token) {
                Ok(msg) => println!("{}", msg),
                Err(e) => {
                    eprintln!("Error: package install failed: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("package") | Some("packages"), Some("remove")) => {
            let name = match target {
                Some(n) => n,
                None => {
                    eprintln!("Error: package remove requires a package name");
                    process::exit(1);
                }
            };
            match client.packages_remove(name, token) {
                Ok(msg) => println!("{}", msg),
                Err(e) => {
                    eprintln!("Error: package remove failed: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("storage") | Some("disk") | Some("disks"), Some("list")) => {
            match client.storage_list() {
                Ok(overview) => {
                    if json_mode {
                        println!("{}", serde_json::to_string_pretty(&overview).unwrap_or_default());
                    } else {
                        println!("=== MOUNTED FILESYSTEMS ===");
                        println!("{:<24} {:<18} {:<10} {:<14} {:<24}", "DEVICE", "MOUNT POINT", "FS TYPE", "SUBVOLUME", "OPTIONS");
                        println!("{:-<24} {:-<18} {:-<10} {:-<14} {:-<24}", "", "", "", "", "");
                        for m in overview.mounts {
                            let sub = m.subvolume.unwrap_or_else(|| "-".to_string());
                            println!("{:<24} {:<18} {:<10} {:<14} {:<24}", m.device, m.mount_point, m.fs_type, sub, m.options);
                        }
                        println!("\n=== BLOCK STORAGE DEVICES ===");
                        println!("{:<16} {:<14} {:<12} {:<12}", "DEVICE", "SIZE", "TYPE", "READ-ONLY");
                        println!("{:-<16} {:-<14} {:-<12} {:-<12}", "", "", "", "");
                        for d in overview.devices {
                            let typ = if d.is_rotational { "HDD" } else { "SSD/NVMe" };
                            let ro = if d.is_read_only { "Yes" } else { "No" };
                            println!("{:<16} {:<14} {:<12} {:<12}", d.name, format_bytes(d.size_bytes), typ, ro);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: failed to query storage: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("storage"), Some("mount")) => {
            let source = match target {
                Some(s) => s,
                None => {
                    eprintln!("Error: storage mount requires source device/path and target mount point");
                    process::exit(1);
                }
            };
            let mount_target = match positional.get(3).map(|s| s.as_str()) {
                Some(t) => t,
                None => {
                    eprintln!("Error: storage mount requires a target mount point");
                    process::exit(1);
                }
            };
            match client.storage_mount(source, mount_target, None, None, token) {
                Ok(msg) => println!("{}", msg),
                Err(e) => {
                    eprintln!("Error: mount failed: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("generation") | Some("generations"), Some("current")) => {
            match client.generation_get_current() {
                Ok(gen) => {
                    if json_mode {
                        println!("{}", serde_json::to_string_pretty(&gen).unwrap_or_default());
                    } else {
                        println!("==================================================");
                        println!("          Mahina Active System Generation");
                        println!("==================================================");
                        println!("  Generation ID:  #{}", gen.id);
                        println!("  Description:    {}", gen.description);
                        println!("  Status:         ACTIVE (Current Boot)");
                        println!("  Healthy:        {}", if gen.is_healthy { "Yes" } else { "No (Watchdog Pending)" });
                        println!("  Kernel Version: {}", gen.kernel_version);
                        println!("==================================================");
                    }
                }
                Err(e) => {
                    eprintln!("Error: failed to query current generation: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("generation") | Some("generations"), Some("list")) => {
            match client.generation_list() {
                Ok(gens) => {
                    if json_mode {
                        println!("{}", serde_json::to_string_pretty(&gens).unwrap_or_default());
                    } else if gens.is_empty() {
                        println!("No generations found.");
                    } else {
                        println!("{:<14} {:<12} {:<10} {:<36}", "GENERATION", "STATUS", "HEALTHY", "DESCRIPTION");
                        println!("{:-<14} {:-<12} {:-<10} {:-<36}", "", "", "", "");
                        for g in gens {
                            let status = if g.is_current { "CURRENT" } else { "AVAILABLE" };
                            let healthy = if g.is_healthy { "Yes" } else { "Pending" };
                            let id_str = format!("#{}", g.id);
                            println!("{:<14} {:<12} {:<10} {:<36}", id_str, status, healthy, g.description);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: failed to list generations: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("generation") | Some("generations"), Some("create")) => {
            let desc = positional.get(2).map(|s| s.as_str()).unwrap_or("Manual snapshot");
            match client.generation_create(desc, token) {
                Ok(gen) => {
                    if json_mode {
                        println!("{}", serde_json::to_string_pretty(&gen).unwrap_or_default());
                    } else {
                        println!("Candidate generation created successfully:");
                        println!("  Generation ID:  #{}", gen.id);
                        println!("  Description:    {}", gen.description);
                        println!("  Status:         STAGED");
                        println!("\nNext step: Run 'mahina-ctl generation activate {}' to schedule for trial boot.", gen.id);
                    }
                }
                Err(e) => {
                    eprintln!("Error: failed to create candidate generation: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("generation") | Some("generations"), Some("activate")) => {
            let id = positional.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or_else(|| {
                eprintln!("Error: 'generation activate' requires a generation ID");
                process::exit(1);
            });
            match client.generation_activate(id, token) {
                Ok(details) => {
                    if json_mode {
                        println!("{}", serde_json::json!({ "status": "activated", "id": id, "details": details }));
                    } else {
                        println!("Generation #{} activated successfully.", id);
                        println!("  Details: {}", details);
                        println!("  Reboot to begin trial boot with boot attempt counter.");
                    }
                }
                Err(e) => {
                    eprintln!("Error: failed to activate candidate generation: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("generation") | Some("generations"), Some("mark-healthy") | Some("mark_healthy")) => {
            let id = positional.get(2).and_then(|s| s.parse::<u32>().ok());
            match client.generation_mark_healthy(id, token) {
                Ok(gen) => {
                    if json_mode {
                        println!("{}", serde_json::to_string_pretty(&gen).unwrap_or_default());
                    } else {
                        println!("Generation #{} verified HEALTHY and promoted to default boot entry.", gen.id);
                        println!("  Description: {}", gen.description);
                    }
                }
                Err(e) => {
                    eprintln!("Error: failed to mark generation healthy: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("generation") | Some("generations"), Some("rollback")) => {
            let target_id = positional.get(2).and_then(|s| s.parse::<u32>().ok());
            match client.generation_rollback(target_id, token) {
                Ok(gen) => {
                    if json_mode {
                        println!("{}", serde_json::to_string_pretty(&gen).unwrap_or_default());
                    } else {
                        println!("Rollback initiated successfully.");
                        println!("  Default boot entry reverted to Generation #{}: {}", gen.id, gen.description);
                    }
                }
                Err(e) => {
                    eprintln!("Error: rollback failed: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("generation") | Some("generations"), Some("pin")) => {
            let id = positional.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or_else(|| {
                eprintln!("Error: 'generation pin' requires a generation ID");
                process::exit(1);
            });
            match client.generation_pin(id, true, token) {
                Ok(details) => println!("{}", details),
                Err(e) => {
                    eprintln!("Error: failed to pin generation: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("generation") | Some("generations"), Some("unpin")) => {
            let id = positional.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or_else(|| {
                eprintln!("Error: 'generation unpin' requires a generation ID");
                process::exit(1);
            });
            match client.generation_pin(id, false, token) {
                Ok(details) => println!("{}", details),
                Err(e) => {
                    eprintln!("Error: failed to unpin generation: {}", e);
                    process::exit(1);
                }
            }
        }
        (Some("generation") | Some("generations"), Some("capabilities")) => {
            match client.generation_capabilities() {
                Ok(caps) => {
                    if json_mode {
                        println!("{}", serde_json::to_string_pretty(&caps).unwrap_or_default());
                    } else {
                        println!("{}", caps.display_summary());
                    }
                }
                Err(e) => {
                    eprintln!("Error: failed to query recovery capabilities: {}", e);
                    process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Unknown command: {}", positional.join(" "));
            print_help();
            process::exit(1);
        }
    }
}
