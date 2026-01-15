# TUI Ether Pinger 🔌

A terminal-based ICMP ping monitoring tool that parses nmap XML output and displays real-time ping statistics in a beautiful interactive interface.

## Features

- Parse nmap XML output files to extract host IP addresses
- Interactive TUI with arrow key navigation
- Multi-host selection with Space bar toggle
- Real-time ping statistics: latency (last, avg, min, max), packet loss
- Color-coded status indicators:
  - Green: Active and responding
  - Red: Recent timeout
  - Yellow: Unreachable (5+ consecutive timeouts)
- Cute RJ45 connector emoji (🔌) for selected/pinging hosts
- Concurrent pinging of multiple hosts
- Clean graceful shutdown

## Prerequisites

ICMP requires raw socket access. You'll need to run the application with elevated privileges:

### Linux/macOS

Option 1: Run with sudo (simplest)
```bash
sudo ./target/release/tui-ether-pinger
```

Option 2: Set capabilities (Linux only, recommended)
```bash
sudo setcap cap_net_raw+ep ./target/release/tui-ether-pinger
./target/release/tui-ether-pinger
```

## Installation

1. Clone this repository
2. Ensure you have Rust installed (https://rustup.rs/)
3. Build the project:
```bash
cargo build --release
```

## Usage

1. First, generate an nmap XML output file:
```bash
nmap 192.168.2.1/24 -p80 -oX output.xml
```

2. Run the TUI pinger:
```bash
# Use default output.xml
sudo ./target/release/tui-ether-pinger

# Or specify a custom XML file
sudo ./target/release/tui-ether-pinger nmapoutput.xml

# Using justfile
just run-xml nmapoutput.xml
```

### Controls

- `↑` / `↓` or `k` / `j` - Navigate up/down through host list
- `Space` - Toggle selection (start/stop pinging)
- `q` or `Esc` - Quit application

### Interface Layout

```
┌─ Hosts ──────────────┬─ Ping Statistics ──────────────────────────────────┐
│                      │                                                    │
│ [ ] 192.168.2.1      │ IP              Status   Last    Avg     Loss Pkts │
│ [x] 192.168.2.3  🔌  │ 192.168.2.3     Active   2.3ms  2.1ms   0%   45/45│
│ [ ] 192.168.2.159    │ 192.168.2.211   Timeout  -      3.2ms   12%  88/100│
│ [x] 192.168.2.211 🔌 │                                                    │
│ [ ] 192.168.2.196    │                                                    │
│                      │                                                    │
└──────────────────────┴────────────────────────────────────────────────────┘
 q: quit | ↑↓: navigate | Space: toggle | 🔌: pinging
```

## Technical Details

### Architecture

- Async runtime: Tokio
- TUI framework: Ratatui
- ICMP library: surge-ping
- XML parsing: quick-xml

### Design

- Each selected host gets its own async ping task
- Ping interval: 1 second
- Ping timeout: 2 seconds
- Statistics use a 100-sample ring buffer for moving averages
- Thread-safe shared state using Arc<RwLock>

### Project Structure

```
src/
├── main.rs     - Entry point, terminal setup, event loop
├── app.rs      - Application state and logic
├── ui.rs       - TUI rendering with ratatui
├── pinger.rs   - ICMP ping async tasks
├── parser.rs   - nmap XML parsing
└── stats.rs    - Ping statistics calculation
```

## Dependencies

- ratatui 0.28 - TUI framework
- crossterm 0.28 - Terminal control
- tokio 1.41 - Async runtime
- surge-ping 0.8 - ICMP pinging
- quick-xml 0.36 - XML parsing
- parking_lot 0.12 - Fast synchronization primitives
- anyhow 1.0 - Error handling
- chrono 0.4 - Time handling
- rand 0.8 - Random number generation

## License

MIT

## Contributing

Pull requests welcome!
