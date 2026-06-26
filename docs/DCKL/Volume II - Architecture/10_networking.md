# LunaOS — Networking Architecture
**Volume II · Chapter 10**
**Classification:** Core Architecture — Network Stack
**Status:** Active · Reference for network service implementation

---

## Purpose

This document specifies the LunaOS networking architecture: the network stack, interface management, DNS configuration, firewall policy, network-related services, and the rules governing what network traffic LunaOS initiates automatically versus what requires explicit user action.

---

## Overview

LunaOS uses a standard Linux network stack managed by NetworkManager in userspace. The design priority for networking is: **user-initiated traffic only**. LunaOS never initiates outbound connections without the user's knowledge or explicit instruction. This is a direct consequence of Core Law II (Local First) and Core Law V (User Owns the Machine).

---

## Design Philosophy

**No automatic outbound traffic.** After a clean boot, LunaOS initiates no outbound network connections automatically. NetworkManager may probe for connectivity (DHCP, DNS), but no LunaOS service contacts remote servers, calls home, checks for updates, or sends telemetry. All outbound traffic from LunaOS infrastructure requires explicit user instruction.

**Cloud is opt-in (Law II).** The network exists because the user may want it — for package updates, cloud bridge AI queries, file downloads. It does not exist to serve LunaOS's infrastructure needs.

**Internal IPC is localhost-only.** All LunaOS inter-process communication that uses TCP/IP is bound to `127.0.0.1`. No LunaOS service opens a port on a network interface. See `07_ipc.md` and `08_security.md`.

---

## Architecture

### Network Stack

```
User Applications
        │ (socket API)
        ▼
Linux Network Stack (kernel)
        │
        ├── IPv4 (CONFIG_INET=y)
        ├── IPv6 (CONFIG_IPV6=y)
        └── netfilter / nftables (CONFIG_NETFILTER=y, CONFIG_NF_TABLES=y)
                │
                ▼
NetworkManager (userspace)
        │
        ├── Wired (ethernet — dhclient or built-in DHCPv4)
        ├── Wireless (Wi-Fi — wpa_supplicant or iwd)
        └── DNS (systemd-resolved equivalent — see DNS section)
```

### Network Services

| Service | Role | Started by |
|---|---|---|
| NetworkManager | Interface management, DHCP, Wi-Fi | luna-init Stage 4 |
| wpa_supplicant or iwd | Wi-Fi authentication | NetworkManager subprocess |
| nftables | Firewall rules | luna-init Stage 4 (before NetworkManager) |

```
TODO:
Decision not yet finalized.
Reason: Wi-Fi authentication backend has not been decided.
Options:
  wpa_supplicant — industry standard, heavy, complex configuration
  iwd (Intel Wireless Daemon) — modern, simpler, better NetworkManager integration
Recommendation: iwd — it is the direction NetworkManager is moving and it is
simpler to own (Law I). Must be a Decision Log entry.
```

### DNS Configuration

```
TODO:
Decision not yet finalized.
Reason: DNS resolver strategy has not been decided.
Options:
  A: NetworkManager writes /etc/resolv.conf with upstream DHCP-provided DNS servers.
     Simple, no local resolver. Relies on ISP DNS. Privacy concerns.
  B: A local caching resolver (Unbound or dnsmasq) sits between NetworkManager
     and the upstream DNS. Provides caching and optional DNS-over-TLS.
  C: systemd-resolved equivalent — not available since LunaOS does not use systemd.
Recommendation: Option B with Unbound for DNS-over-TLS support.
This is a privacy-aligned choice consistent with Law II (Local First).
Must be a Decision Log entry before networking is implemented.
```

### Firewall Architecture

The firewall is implemented via nftables (see `08_security.md` for the full ruleset). From the network architecture perspective:

**Default inbound policy: DROP** (except established connections and loopback)
**Default outbound policy: ACCEPT** (per-application restriction via AppArmor, not firewall rules)

The firewall is a luna-init service at Stage 4, loaded before NetworkManager. This ensures the firewall is active before any network interface comes up.

```
Luna-init Stage 4 service order:
  1. nftables (firewall rules loaded)
  2. NetworkManager (interfaces come up — firewall already active)
```

This order is mandatory. Reversing it would create a window where the network is active without firewall protection.

---

## Technical Details

### NetworkManager Configuration

NetworkManager configuration lives in `/etc/NetworkManager/`:

```
/etc/NetworkManager/
├── NetworkManager.conf          # Main configuration
└── system-connections/          # Saved connection profiles (TOML-like keyfile format)
    └── home-wifi.nmconnection   # Example saved Wi-Fi profile
```

```
TODO:
Decision not yet finalized.
Reason: NetworkManager uses its own keyfile format, not TOML.
DL-008 specifies TOML for all LunaOS configuration files.
NetworkManager's keyfile format is not TOML-compatible.
Options:
  A: Accept NetworkManager's keyfile format as an upstream exception (Law I permits
     using upstream tools we understand).
  B: Write a configuration translator: user edits TOML, a service converts to
     NetworkManager keyfile format.
Option A is simpler and more maintainable. NetworkManager is a well-understood tool.
The Law I exception for upstream tools that are fully understood applies here.
This must be a documented exception in the Decision Log.
```

**NetworkManager.conf:**

```ini
[main]
plugins = keyfile
dns = none               # LunaOS manages DNS separately (via Unbound — TODO)

[connection]
wifi.powersave = 2       # Disable Wi-Fi power saving for lower latency

[logging]
backend = file
level = WARN
```

### Interface Naming

LunaOS uses kernel-assigned interface names (e.g., `eth0`, `wlan0`) rather than predictable network interface names (`enp3s0`, `wlp2s0`). Predictable naming requires `udev` rules and adds complexity without a clear benefit for the LunaOS use case.

```
TODO:
Decision not yet finalized.
Reason: Interface naming convention (kernel names vs. predictable names) has
not been formally decided.
Predictable names are the modern default and NetworkManager works well with them.
Kernel names are simpler and require no udev rules.
Since LunaOS does not use udev (udevd would be a system service dependency —
check whether devtmpfs is sufficient), predictable names may not be achievable
without additional tooling.
This must be resolved as part of the device management architecture.
```

### Network-Related sysctl Settings

```toml
# /etc/luna/sysctl.toml
[net.core]
rmem_max          = 134217728    # Maximum receive socket buffer (128 MB)
wmem_max          = 134217728    # Maximum send socket buffer (128 MB)
netdev_max_backlog = 5000        # Increase network device input queue

[net.ipv4]
tcp_fastopen       = 3           # Enable TCP Fast Open for clients and servers
tcp_congestion_control = "bbr"   # BBR congestion control — better throughput
tcp_notsent_lowat  = 16384       # Reduce TCP send buffer latency

[net.ipv6]
disable_ipv6 = 0                 # IPv6 enabled (not disabled)
```

BBR congestion control (`net.ipv4.tcp_congestion_control = bbr`) provides better throughput and lower latency than the default CUBIC, especially useful when `lpkg` is downloading packages or Ollama is pulling model weights.

### Outbound Traffic Policy

This table defines exactly what outbound network traffic LunaOS initiates and under what conditions:

| Traffic | Initiator | Trigger | User Control |
|---|---|---|---|
| DHCP | NetworkManager | Network interface comes up | Automatic — required for network access |
| DNS queries | NetworkManager / Unbound | Any DNS lookup | Automatic — required for network access |
| NTP (clock sync) | TODO — see Open Questions | Boot or periodic | Should be user-configurable |
| Package index | lpkg | `lpkg update` (manual) | ✅ Manual only |
| Package download | lpkg | `lpkg install <pkg>` (manual) | ✅ Manual only |
| Model download | Ollama | `ollama pull <model>` (manual) | ✅ Manual only |
| Cloud bridge AI | luna-ai-d | `luna bridge --send` (manual) | ✅ Manual, explicit opt-in |
| Crash reports | — | Never | ❌ Does not exist |
| Telemetry | — | Never | ❌ Does not exist |
| Auto-updates | — | Never | ❌ LunaOS never auto-updates |

```
TODO:
Decision not yet finalized.
Reason: NTP (time synchronization) has not been specified.
Options:
  A: Use the system clock set at boot from hwclock only — no NTP.
     Simple. Correct on hardware with accurate clocks.
  B: Run chronyd or ntpd — automatic continuous NTP sync.
     More accurate. An outbound connection that runs without user instruction.
  C: Run ntpd only when the user explicitly requests a clock sync.
     Best for Law II compliance.
For Law II compliance, Option C or a strictly controlled Option B
(only syncs on first boot and manual request) is preferred.
Must be a Decision Log entry.
```

### LAN and Local Network

LunaOS makes no assumptions about the local network topology. It does not:
- Run mDNS/Avahi by default (zero-configuration networking — opt-in)
- Run SSH server by default (must be explicitly installed and enabled)
- Run any other listening service on network interfaces

After a clean install, `nmap` scanning the LunaOS host from another machine on the LAN should show no open ports.

---

## Future Improvements

| Improvement | Target | Notes |
|---|---|---|
| iwd adoption decision | v1 | Replace wpa_supplicant if iwd is chosen |
| Unbound DNS with DNS-over-TLS | v1 | Privacy-aligned DNS resolver |
| NTP policy Decision Log entry | v1 | Decide NTP behavior |
| NetworkManager TOML wrapper | v2 | If TOML consistency is required |
| `luna network` CLI | v1 | Status, connect, disconnect commands wrapping nmcli |
| Wi-Fi captive portal handling | v1.5 | Detect and handle captive portals |
| VPN support | v1.5 | OpenVPN or WireGuard via NetworkManager plugin |
| mDNS opt-in | v1.5 | Avahi disabled by default, enabled via luna network --mdns on |

---

## Open Questions

```
TODO:
Decision not yet finalized.
```

1. **Wi-Fi backend.** wpa_supplicant vs. iwd. Must be a Decision Log entry.

2. **DNS resolver.** NetworkManager passthrough vs. local Unbound. Must be a Decision Log entry. Privacy-sensitive decision.

3. **NTP synchronization policy.** Automatic vs. manual-only vs. first-boot-only. Must be a Decision Log entry.

4. **Interface naming.** Kernel names vs. predictable names. Depends on udev/devtmpfs device management decision.

5. **udev.** Whether LunaOS runs udevd or relies on devtmpfs alone for device management has not been decided. This affects interface naming, hotplug events, and device node permissions.

6. **NetworkManager TOML exception.** If NetworkManager's keyfile format is accepted as an upstream exception to DL-008, this must be documented in the Decision Log.

---

## AI Context

An AI agent implementing LunaOS networking must understand:

- After a clean boot with no user interaction, LunaOS initiates no outbound connections except DHCP and DNS (required for network functionality). Everything else is user-triggered.
- No LunaOS service opens a port on a network interface. `localhost:7734` and `localhost:11434` are loopback only. External port scans of a LunaOS host show no open ports.
- The firewall is loaded before NetworkManager in the service startup order. This is mandatory — do not change this order.
- `luna-ai-d` never makes outbound calls except (a) localhost, and (b) cloud bridge via explicit user command. If implementing luna-ai-d, do not add any automatic outbound calls.
- The NTP strategy is undecided. Do not implement a background NTP sync without a Decision Log entry. Until decided, the clock is set from hwclock at boot.
- DNS resolution depends on whether Unbound is adopted. Until that decision is made, NetworkManager writes to `/etc/resolv.conf` directly. Do not hardcode DNS resolver paths.
- Auto-updates do not exist in LunaOS. Any code that schedules automatic downloads or package updates is a violation of Core Law V.

---

*Document: `Volume II / 10_networking.md`*
*Author: Hardik Bhaskar (Luna Kitsune)*
*Version: 0.1-draft*
*Depends on: architecture_overview.md, linux_architecture.md, init_system.md, security.md, ipc.md, core_laws.md (Law II, V), non_negotiables.md*
