# TUI Ether Pinger - Complete Feature List

## Overview
A dual-mode network monitoring tool with terminal user interface, supporting both ICMP ping and HTTP health checking.

## Modes

### ICMP Ping Mode (Default)
- Real-time ICMP echo request/reply monitoring
- Latency tracking: last, average, minimum, maximum
- Packet loss percentage calculation
- Status indicators: Active, Timeout, Unreachable (5+ consecutive failures)
- 1-second ping interval, 2-second timeout per ping
- Requires raw socket access (sudo or CAP_NET_RAW)

### HTTP Mode (`--http`)
- HTTP GET request health checking
- Configurable port (default: 80)
- Response time tracking: last, average, minimum, maximum
- HTTP status code display (200 OK, 404 Not Found, 500 Server Error, etc.)
- Content size reporting
- Error message capture and display
- Success rate calculation
- 1-second request interval, 5-second timeout per request
- Does not follow redirects (shows 3xx status)

## Interface Features

### Host List
- Parse nmap XML files to extract IP addresses
- Interactive list with arrow key navigation (↑↓ or k/j)
- Visual selection indicators with checkboxes `[x]` and `[ ]`
- Cute RJ45 connector emoji (🔌) for actively monitored hosts
- Host counter showing selected/total (e.g., "Hosts (3/12)")
- Current selection highlighted with dark gray background

### Statistics Panel
- Real-time updating statistics for all selected hosts
- Color-coded status indicators:
  - **ICMP Mode**: Green (Active), Red (Timeout), Yellow (Unreachable)
  - **HTTP Mode**: Green (2xx Success), Yellow (4xx Client Error), Red (5xx/Network Error)
- Mode-specific columns:
  - **ICMP**: IP, Status, Last, Avg, Loss %, Packets
  - **HTTP**: IP, Status, Last, Avg, Size, Error

### Status Bar
- Current mode display (ICMP or HTTP:PORT)
- Pause indicator (⏸ PAUSED in yellow when paused)
- Complete keyboard shortcuts reference
- Real-time mode and state feedback

## User Controls

### Navigation
- `↑` / `↓` - Move selection up/down through host list
- `k` / `j` - Vim-style navigation (up/down)

### Selection
- `Space` - Toggle monitoring for currently selected host
- `a` - Select all hosts (starts monitoring if not paused)
- `d` - Deselect all hosts (stops all monitoring)

### Monitoring Control
- `p` - Pause/resume all monitoring
  - Paused state stops all active tasks
  - Resume restarts all previously selected hosts
  - Works in both ICMP and HTTP modes

### Data Export
- `s` - Export current statistics to CSV file
  - Timestamped filename: `stats_export_YYYYMMDD_HHMMSS.csv`
  - Mode-aware column structure
  - Includes all hosts (selected and unselected)
  - Can be exported while monitoring is active

### Application
- `q` or `Esc` - Graceful shutdown (stops all tasks, restores terminal)

## CLI Options

### Flags
- `--http` - Enable HTTP checking mode (default: ICMP ping)
- `--port PORT` - Specify port for HTTP mode (default: 80)
- `-h` / `--help` - Display comprehensive help message

### Arguments
- `[XML_FILE]` - Path to nmap XML output file (default: output.xml)

### Examples
```bash
# ICMP ping mode (default)
sudo ./tui-ether-pinger

# HTTP mode on port 80
sudo ./tui-ether-pinger --http

# HTTP mode on port 8080
sudo ./tui-ether-pinger --http --port 8080

# Custom XML file
sudo ./tui-ether-pinger --http --port 443 scan_results.xml

# Show help
./tui-ether-pinger --help
```

## Technical Features

### Async Architecture
- Tokio async runtime for concurrent operations
- One async task per selected host
- Independent task lifecycle management
- Graceful task cancellation on deselection/pause
- Thread-safe shared state using Arc<RwLock>

### Statistics Engine
- 100-sample ring buffer for moving averages
- Real-time calculation of min/max/avg
- Packet loss and success rate percentages
- Consecutive timeout tracking
- Timestamp tracking for last update

### UI Rendering
- Ratatui framework for terminal UI
- Mode-aware rendering (different panels for ICMP vs HTTP)
- 50ms refresh rate for smooth updates
- Crossterm for cross-platform terminal control
- Smart rendering (only updates changed cells)

### Error Handling
- Terminal state restoration on panic
- Graceful handling of network errors
- Permission error detection and reporting
- XML parsing error messages with context
- Invalid CLI argument feedback

### Data Persistence
- CSV export with proper escaping
- Mode-aware column structure
- Timestamp-based filenames
- Human-readable format

## File Structure

### Source Files
- `main.rs` - CLI parsing, terminal setup, event loop
- `app.rs` - Application state, mode handling, task management
- `ui.rs` - TUI rendering, mode-aware statistics panels
- `pinger.rs` - ICMP ping implementation (surge-ping)
- `http_checker.rs` - HTTP checking implementation (reqwest)
- `parser.rs` - nmap XML parsing (quick-xml)
- `stats.rs` - AppMode enum, Stats wrapper, PingStats
- `http_stats.rs` - HTTP-specific statistics

### Configuration
- `Cargo.toml` - Dependencies and project metadata
- `justfile` - Development task automation
- `.gitignore` - Ignore build artifacts and exports

### Documentation
- `README.md` - User guide and quick start
- `PLAN.md` - Architecture and design documentation
- `FEATURES.md` - This file - comprehensive feature list

## Justfile Targets

### ICMP Mode
- `just run` - Run debug build (default mode)
- `just run-release` - Run release build
- `just run-xml FILE` - Run with custom XML file

### HTTP Mode
- `just run-web-80` - HTTP mode on port 80 (debug)
- `just run-web-80-release` - HTTP mode on port 80 (release)
- `just run-web-port PORT` - HTTP mode on custom port
- `just run-web-80-xml FILE` - HTTP mode with custom XML

### Development
- `just build` - Build debug version
- `just release` - Build release version
- `just install` - Build release and set capabilities
- `just ci` - Run all CI checks (format, clippy, test, build)
- `just dev` - Quick development checks (format, check, test)
- `just watch` - Rebuild on file changes

### Utilities
- `just nmap-scan NETWORK` - Generate nmap XML
- `just doc` - Generate and open documentation
- `just clean` - Remove build artifacts

## Dependencies

### Core
- **ratatui 0.28** - TUI framework
- **crossterm 0.28** - Terminal manipulation
- **tokio 1.41** - Async runtime (full features)

### Networking
- **surge-ping 0.8** - ICMP ping (raw sockets)
- **reqwest 0.12** - HTTP client (rustls-tls)

### Data Processing
- **quick-xml 0.36** - XML parsing
- **parking_lot 0.12** - Fast locks (RwLock)
- **chrono 0.4** - Timestamps and formatting

### Utilities
- **anyhow 1.0** - Error handling
- **rand 0.8** - Random number generation

## Platform Support

### Linux
- Full support for ICMP and HTTP modes
- Requires sudo or `setcap cap_net_raw+ep`
- Tested on various distributions

### macOS
- Full support for ICMP and HTTP modes
- Requires sudo for ICMP
- HTTP mode works without elevated privileges

### Windows
- HTTP mode: Full support
- ICMP mode: May require administrator privileges
- surge-ping provides Windows raw socket support

## Security Considerations

### ICMP Mode
- Requires raw socket access (CAP_NET_RAW)
- Recommendation: Use `setcap` instead of sudo for production
- Never expose raw socket capability unnecessarily

### HTTP Mode
- Uses rustls (Rust TLS) instead of system OpenSSL
- Does not follow redirects (prevents SSRF)
- 5-second timeout prevents hanging connections
- Errors are captured but sanitized for display

### General
- No network credentials handled
- No sensitive data stored
- CSV exports contain only statistics, not packet data
- Terminal restoration on crash prevents state corruption

## Future Enhancement Ideas

### Features
- [ ] Multiple concurrent XML file support
- [ ] Real-time graph visualization of latency/response times
- [ ] Alerting on threshold breaches
- [ ] Custom ping/request intervals per host
- [ ] Filter/search hosts by IP or pattern
- [ ] DNS hostname resolution and display
- [ ] HTTPS support (port 443 with TLS verification)
- [ ] Custom HTTP headers and request methods
- [ ] Configurable timeout and interval values
- [ ] JSON export format option
- [ ] Background mode (daemon) with log file output

### UI Improvements
- [ ] Split screen for comparing multiple hosts
- [ ] Help overlay (press 'h' for key bindings)
- [ ] Confirmation dialog for destructive actions
- [ ] Status messages for export/actions
- [ ] Mouse support for clicking hosts
- [ ] Resizable panels
- [ ] Themes/color schemes

### Performance
- [ ] Lazy rendering (only on state change)
- [ ] Configurable ring buffer size
- [ ] Memory usage optimization for large host lists
- [ ] Batch statistics updates

### Integration
- [ ] Prometheus metrics export
- [ ] InfluxDB time-series export
- [ ] Webhook notifications
- [ ] Integration with monitoring systems

## Version History

### v0.1.0 - Initial Release
- Dual-mode operation (ICMP/HTTP)
- Interactive TUI with host selection
- Real-time statistics
- Pause/resume functionality
- Select/deselect all
- CSV statistics export
- Comprehensive CLI help
- Port specification for HTTP mode
- Host counter display
- Cross-platform support
