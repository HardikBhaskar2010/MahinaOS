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
         COMMANDS:\n  \
         system state            Query holistic system telemetry and state\n  \
         system reboot           Initiate authorized system reboot\n  \
         system shutdown         Initiate authorized system shutdown\n",
        DEFAULT_SOCKET_PATH
    );
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

    match (positional.first().map(|s| s.as_str()), positional.get(1).map(|s| s.as_str())) {
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
        _ => {
            eprintln!("Unknown command: {}", positional.join(" "));
            print_help();
            process::exit(1);
        }
    }
}
