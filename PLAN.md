# TUI Ether Pinger - Design & Implementation Plan

## Project Overview

A Rust-based terminal user interface (TUI) application for ICMP ping monitoring of hosts discovered via nmap. The application provides real-time statistics and an interactive interface with multi-host selection.

## Requirements

1. Parse nmap XML output to extract IP addresses
2. Interactive TUI with arrow key navigation and Space bar selection
3. Continuous ICMP pinging with live statistics
4. Multi-host concurrent monitoring
5. Color-coded status indicators
6. Visual indicator (RJ45 emoji) for active pings

## Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────────────┐
│                         Main Thread                         │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │  Terminal   │  │  Event Loop  │  │   UI Renderer    │  │
│  │   Setup     │→ │   (50ms)     │→ │   (ratatui)      │  │
│  └─────────────┘  └──────────────┘  └──────────────────┘  │
│         ↑                │                      ↑           │
│         │                └──────────────────────┘           │
│         └────────────── Keyboard Events ───────────────────┘│
└─────────────────────────────────────────────────────────────┘
                             │
                             ↓
              ┌──────────────────────────────┐
              │   Shared State (RwLock)      │
              │  HashMap<IP, PingStats>      │
              └──────────────────────────────┘
                   ↑           ↑           ↑
                   │           │           │
        ┌──────────┴───┬───────┴────┬──────┴────────┐
        │              │            │               │
   ┌────▼─────┐  ┌────▼─────┐ ┌────▼─────┐   ┌─────▼────┐
   │  Ping    │  │  Ping    │ │  Ping    │...│  Ping    │
   │ Task 1   │  │ Task 2   │ │ Task 3   │   │ Task N   │
   │(Tokio)   │  │(Tokio)   │ │(Tokio)   │   │(Tokio)   │
   └──────────┘  └──────────┘ └──────────┘   └──────────┘
```

### Data Flow

1. **Initialization**: Parse nmap XML → Extract IPs → Initialize App state
2. **User Input**: Keyboard events → Update selection → Spawn/stop ping tasks
3. **Ping Tasks**: Async ICMP requests → Update shared statistics → Continue loop
4. **Rendering**: Read shared stats → Format UI → Draw to terminal
5. **Shutdown**: Signal all tasks → Wait for cleanup → Restore terminal

## Module Breakdown

### 1. parser.rs

**Purpose**: Parse nmap XML files to extract IP addresses

**Implementation**:
- Uses `quick-xml` for efficient streaming XML parsing
- Searches for `<hosthint>` elements containing `<address addr="IP">` tags
- Returns `Vec<IpAddr>` or error if parsing fails

**Key Functions**:
- `parse_nmap_xml(path: &str) -> Result<Vec<IpAddr>>`

### 2. stats.rs

**Purpose**: Manage ping statistics and status

**Data Structures**:
```rust
PingStatus {
    NotStarted,
    Active,      // Receiving responses
    Timeout,     // Recent timeout
    Unreachable, // 5+ consecutive timeouts
}

PingStats {
    status: PingStatus,
    last_latency: Option<Duration>,
    avg_latency: Option<Duration>,
    min_latency: Option<Duration>,
    max_latency: Option<Duration>,
    packets_sent: u64,
    packets_received: u64,
    packet_loss_percent: f64,
    recent_latencies: VecDeque<Duration>,  // Ring buffer (100 samples)
    consecutive_timeouts: u32,
}
```

**Key Methods**:
- `update(result: Option<Duration>)` - Update stats with new ping result
- `calculate_stats()` - Compute avg/min/max from ring buffer

### 3. pinger.rs

**Purpose**: Async ICMP ping tasks

**Implementation**:
- Uses `surge-ping` for cross-platform ICMP support
- Each task pings one IP every 1 second
- 2-second timeout per ping
- Updates shared statistics on each result
- Listens for shutdown signal via `tokio::sync::watch`

**Key Functions**:
- `start_ping_task(ip, stats, shutdown_rx)` - Main async ping loop

**Task Lifecycle**:
1. Create ICMP client and pinger
2. Enter loop:
   - Send ICMP echo request
   - Wait for response with timeout
   - Update shared stats (acquire write lock)
   - Sleep until next interval
   - Check shutdown signal
3. Exit on shutdown

### 4. app.rs

**Purpose**: Application state and business logic

**Data Structures**:
```rust
Host {
    ip: IpAddr,
    selected: bool,
}

App {
    hosts: Vec<Host>,
    selected_index: usize,
    ping_stats: Arc<RwLock<HashMap<IpAddr, PingStats>>>,
    ping_handles: HashMap<IpAddr, JoinHandle<()>>,
    shutdown_senders: HashMap<IpAddr, watch::Sender<bool>>,
    should_quit: bool,
}
```

**Key Methods**:
- `new(ips)` - Initialize app with parsed IPs
- `handle_key(key)` - Process keyboard input
- `move_selection(delta)` - Navigate host list
- `toggle_selection()` - Start/stop pinging selected host
- `start_ping_task(ip)` - Spawn async ping task
- `stop_ping_task(ip)` - Signal and abort ping task
- `shutdown()` - Stop all tasks gracefully

**State Management**:
- `Arc<RwLock>` for thread-safe shared statistics
- Read locks in UI thread (frequent, fast)
- Write locks in ping tasks (infrequent, per-ping)

### 5. ui.rs

**Purpose**: TUI rendering with ratatui

**Layout Design**:
```
┌─────────────────────────────────────────────────────────┐
│  Main Container (Vertical Split)                        │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Top Section (Horizontal Split 30/70)            │  │
│  │  ┌─────────────┬──────────────────────────────┐  │  │
│  │  │  Host List  │  Statistics Table            │  │  │
│  │  │             │                              │  │  │
│  │  └─────────────┴──────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Help Bar                                         │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

**Key Functions**:
- `render(app, frame)` - Main rendering entry point
- `render_host_list(app, frame, area)` - Draw host list with checkboxes and RJ45
- `render_stats_panel(app, frame, area)` - Draw statistics table
- `render_help(frame, area)` - Draw help text

**Visual Features**:
- Checkboxes: `[x]` selected, `[ ]` unselected
- RJ45 emoji: `🔌` shown for selected hosts
- Highlighted row: Dark gray background for current selection
- Color coding: Green=Active, Red=Timeout, Yellow=Unreachable
- Bold headers with underline

### 6. main.rs

**Purpose**: Application entry point and event loop

**Flow**:
1. Setup panic hook (restore terminal on crash)
2. Parse nmap XML
3. Create App instance
4. Initialize terminal (raw mode, alternate screen)
5. Enter event loop:
   - Render UI
   - Poll for keyboard events (50ms timeout)
   - Handle events
   - Check quit flag
6. Cleanup:
   - Shutdown all ping tasks
   - Restore terminal
   - Exit

**Terminal Management**:
- Enable raw mode (disable line buffering, echo)
- Enter alternate screen (preserve shell state)
- Panic hook ensures terminal restoration on crash

## Concurrency Model

### Thread Safety

**Problem**: Multiple async tasks writing statistics while UI thread reads

**Solution**: `Arc<RwLock<HashMap<IpAddr, PingStats>>>`
- Multiple readers (UI rendering) can access simultaneously
- Single writer (ping task) gets exclusive access
- `parking_lot::RwLock` for better performance than std

### Task Management

**Spawning**:
1. User selects host (Space bar)
2. Create watch channel for shutdown signal
3. Spawn tokio task with cloned Arc
4. Store JoinHandle and shutdown Sender

**Stopping**:
1. User deselects host
2. Send `true` via shutdown channel
3. Abort JoinHandle
4. Remove from tracking maps

### Graceful Shutdown

On quit:
1. Send shutdown signal to all tasks
2. Wait 100ms for cleanup
3. Exit (remaining tasks aborted by tokio runtime drop)

## Error Handling

### Parsing Errors
- File not found → Context message
- Invalid XML → Position and error description
- No hosts found → Early exit with message

### Ping Errors
- Permission denied → stderr message (need sudo/capabilities)
- Network unreachable → Recorded as timeout
- Invalid IP → Skipped during parsing

### Terminal Errors
- Setup failure → Return error to main
- Panic during execution → Panic hook restores terminal

## Performance Considerations

### Memory
- Ring buffer capped at 100 samples per host
- ~800 bytes per host for statistics
- Example: 100 hosts = ~80KB

### CPU
- UI renders at ~20 FPS (50ms event poll)
- ratatui smart rendering (only changed cells)
- Ping tasks sleep between attempts

### Lock Contention
- Read locks frequent but fast (UI rendering)
- Write locks infrequent (1/sec per host)
- Use `try_read()` in UI to avoid blocking

## Testing Strategy

### Unit Tests
- `parser::parse_nmap_xml()` with sample XML
- `PingStats::update()` for stat calculations
- `App::move_selection()` boundary conditions

### Integration Tests
- Full workflow with mock XML
- Terminal setup/teardown
- Keyboard event handling

### Manual Testing
1. Parse real nmap output
2. Select multiple hosts
3. Verify ping statistics update
4. Test navigation
5. Test color coding (disconnect network)
6. Test graceful shutdown
7. Test permissions error

## Future Enhancements

### Configuration
- CLI args for XML path, ping interval, timeout
- Config file (~/.config/tui-ether-pinger.toml)
- Custom packet size

### Features
- Save/load host selections
- Export statistics to CSV
- DNS hostname resolution
- Graph of latency over time
- Sound alerts on host unreachable
- Filter/search hosts
- Custom ping count before status change

### UI Improvements
- Resizable panels
- Sort by latency/status
- Mouse support
- Themes

### Performance
- Batch statistics updates
- Lazy rendering (only on change)
- Optimize lock granularity

## Dependencies Rationale

| Crate | Version | Purpose | Alternative |
|-------|---------|---------|-------------|
| ratatui | 0.28 | TUI framework | cursive, tui-rs |
| crossterm | 0.28 | Terminal control | termion |
| tokio | 1.41 | Async runtime | async-std |
| surge-ping | 0.8 | ICMP | pnet (more complex) |
| quick-xml | 0.36 | XML parsing | serde-xml-rs (slower) |
| parking_lot | 0.12 | Fast locks | std::sync |
| anyhow | 1.0 | Error handling | thiserror |
| chrono | 0.4 | Time | std::time |
| rand | 0.8 | RNG | getrandom |

## Build & Distribution

### Build Profiles
- Debug: Fast compile, larger binary, debug symbols
- Release: Optimized, smaller binary, no debug symbols

### Cross-Platform Support
- Linux: Tested, requires capabilities
- macOS: Should work, requires sudo
- Windows: surge-ping supports, may need admin

### Binary Size
- Release build: ~5-8 MB (with dependencies)
- Strip symbols: `strip target/release/tui-ether-pinger`
- Potential: ~3-4 MB stripped

## Implementation Checklist

- [x] Project setup (Cargo.toml)
- [x] XML parser implementation
- [x] Statistics module
- [x] Ping task implementation
- [x] Application state management
- [x] UI rendering
- [x] Event loop and main
- [x] Build and compile
- [x] Documentation (README.md)
- [x] Design documentation (PLAN.md)
- [ ] Unit tests
- [ ] Integration tests
- [ ] Manual testing with real network
- [ ] Performance profiling
- [ ] Release build optimization
