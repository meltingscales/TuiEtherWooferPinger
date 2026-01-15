# TUI Ether Pinger - justfile
# https://github.com/casey/just

# Default recipe to display help
default:
    @just --list

# Build the project in debug mode
build:
    cargo build

# Build the project in release mode
release:
    cargo build --release

# Run the application (debug mode, requires sudo)
run:
    sudo cargo run

# Run the application (release mode, requires sudo)
run-release:
    sudo ./target/release/tui-ether-pinger

# Run with a custom XML file
run-xml FILE:
    sudo cargo run -- {{FILE}}

# Check the project for errors without building
check:
    cargo check

# Run clippy for linting
clippy:
    cargo clippy --all-targets --all-features

# Format the code
fmt:
    cargo fmt

# Format check (CI-friendly)
fmt-check:
    cargo fmt -- --check

# Clean build artifacts
clean:
    cargo clean

# Run tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Set capabilities on the binary (Linux only, allows non-root execution)
set-caps:
    sudo setcap cap_net_raw+ep ./target/release/tui-ether-pinger

# Build release and set capabilities
install: release set-caps
    @echo "Binary ready at ./target/release/tui-ether-pinger"
    @echo "You can now run it without sudo!"

# Generate nmap output for testing
nmap-scan NETWORK="192.168.1.0/24":
    nmap {{NETWORK}} -p80 -oX output.xml
    @echo "Created output.xml with scan results"

# Quick nmap scan (faster, less thorough)
nmap-quick NETWORK="192.168.1.0/24":
    nmap {{NETWORK}} -sn -oX output.xml
    @echo "Created output.xml with quick scan results"

# Watch and rebuild on changes
watch:
    cargo watch -x build

# Watch and run on changes (requires cargo-watch and sudo)
watch-run:
    sudo cargo watch -x run

# Generate documentation
doc:
    cargo doc --no-deps --open

# Check dependencies for updates
outdated:
    cargo outdated

# Update dependencies
update:
    cargo update

# Audit dependencies for security vulnerabilities
audit:
    cargo audit

# Full CI check (format, clippy, test, build)
ci: fmt-check clippy test release
    @echo "✓ All CI checks passed!"

# Show binary size
size:
    @ls -lh target/release/tui-ether-pinger 2>/dev/null || echo "Release binary not built yet. Run 'just release' first."

# Strip binary to reduce size
strip:
    strip target/release/tui-ether-pinger
    @just size

# Dev workflow: format, check, test
dev: fmt check test
    @echo "✓ Dev checks passed!"

# Full build workflow: clean, format, check, test, release
all: clean fmt check test release
    @echo "✓ Full build complete!"
