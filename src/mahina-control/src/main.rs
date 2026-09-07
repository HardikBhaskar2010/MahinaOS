use mahina_control::audit::AuditLogger;
use mahina_control::daemon::{ControlDaemon, DEFAULT_SOCKET_PATH};
use mahina_control::idempotency::IdempotencyStore;
use mahina_control::providers::ProviderDispatcher;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;

fn print_usage(program: &str) {
    eprintln!(
        "MahinaOS System Control Plane Daemon (mahina-control-d)\n\n\
         Usage:\n  \
         {} [OPTIONS]\n\n\
         Options:\n  \
         -s, --socket <PATH>     Path to Unix domain socket (default: {})\n  \
         -a, --audit-log <PATH>  Path to audit log file (default: /var/log/mahina/control-audit.log)\n  \
         -h, --help              Print help information\n  \
         -v, --version           Print version information\n",
        program, DEFAULT_SOCKET_PATH
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let program = args.first().map(|s| s.as_str()).unwrap_or("mahina-control-d");

    let mut socket_path = PathBuf::from(DEFAULT_SOCKET_PATH);
    let mut audit_path: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_usage(program);
                return Ok(());
            }
            "-v" | "--version" => {
                println!("mahina-control-d {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "-s" | "--socket" => {
                if i + 1 < args.len() {
                    socket_path = PathBuf::from(&args[i + 1]);
                    i += 1;
                } else {
                    eprintln!("Error: --socket requires a path argument");
                    std::process::exit(1);
                }
            }
            "-a" | "--audit-log" => {
                if i + 1 < args.len() {
                    audit_path = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                } else {
                    eprintln!("Error: --audit-log requires a path argument");
                    std::process::exit(1);
                }
            }
            other => {
                eprintln!("Unknown option: {}", other);
                print_usage(program);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let audit_logger = if let Some(p) = audit_path {
        Arc::new(AuditLogger::with_file_path(p))
    } else {
        Arc::new(AuditLogger::new())
    };

    let dispatcher = Arc::new(ProviderDispatcher::new());
    let idempotency = Arc::new(IdempotencyStore::new());

    let daemon = ControlDaemon::with_components(
        &socket_path,
        dispatcher,
        idempotency,
        audit_logger,
    );

    println!(
        "mahina-control-d: initializing MahinaOS System Control Plane v{}",
        env!("CARGO_PKG_VERSION")
    );
    println!("mahina-control-d: listening on {:?}", socket_path);

    if let Err(e) = daemon.listen() {
        eprintln!("mahina-control-d fatal error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
