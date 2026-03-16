# Default: list available commands
default:
    @just --list

# ── Development ───────────────────────────────────────────────────────────────

# Run in debug mode
run:
    cargo run

# Run with logging enabled
run-log:
    RUST_LOG=cosmic_browser=debug cargo run

# Build debug
build:
    cargo build

# Build release
build-release:
    cargo build --release

# Check without building
check:
    cargo check

# Run clippy
lint:
    cargo clippy -- -D warnings

# Format code
fmt:
    cargo fmt

# Format check (CI)
fmt-check:
    cargo fmt -- --check

# Run all checks (fmt + clippy + test)
ci: fmt-check lint test

# ── Testing ───────────────────────────────────────────────────────────────────

# Run tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# ── Dependencies ──────────────────────────────────────────────────────────────

# Install system dependencies (Debian/Ubuntu)
deps-ubuntu:
    sudo apt install -y \
        libwebkit2gtk-4.1-dev \
        libgtk-3-dev \
        libayatana-appindicator3-dev \
        librsvg2-dev \
        pkg-config \
        build-essential

# Install system dependencies (Fedora)
deps-fedora:
    sudo dnf install -y \
        webkit2gtk4.1-devel \
        gtk3-devel \
        librsvg2-devel \
        pkgconf-pkg-config

# Install system dependencies (Arch)
deps-arch:
    sudo pacman -S --needed \
        webkit2gtk-4.1 \
        gtk3 \
        librsvg \
        pkgconf \
        base-devel

# ── Installation ──────────────────────────────────────────────────────────────

# Install to ~/.local
install: build-release
    install -Dm755 target/release/cosmic-browser \
        ~/.local/bin/cosmic-browser
    install -Dm644 data/com.system76.CosmicBrowser.desktop \
        ~/.local/share/applications/com.system76.CosmicBrowser.desktop
    install -Dm644 data/icons/com.system76.CosmicBrowser.svg \
        ~/.local/share/icons/hicolor/scalable/apps/com.system76.CosmicBrowser.svg
    @echo "Installed to ~/.local"

# Uninstall from ~/.local
uninstall:
    rm -f ~/.local/bin/cosmic-browser
    rm -f ~/.local/share/applications/com.system76.CosmicBrowser.desktop
    rm -f ~/.local/share/icons/hicolor/scalable/apps/com.system76.CosmicBrowser.svg
    @echo "Uninstalled"

# Install system-wide (requires sudo)
install-system: build-release
    sudo install -Dm755 target/release/cosmic-browser \
        /usr/local/bin/cosmic-browser
    sudo install -Dm644 data/com.system76.CosmicBrowser.desktop \
        /usr/local/share/applications/com.system76.CosmicBrowser.desktop
    sudo install -Dm644 data/icons/com.system76.CosmicBrowser.svg \
        /usr/local/share/icons/hicolor/scalable/apps/com.system76.CosmicBrowser.svg
    @echo "Installed system-wide"

# ── Flatpak ───────────────────────────────────────────────────────────────────

# Build Flatpak bundle
flatpak-build:
    flatpak-builder \
        --force-clean \
        --user \
        build-flatpak \
        data/com.system76.CosmicBrowser.json

# Install Flatpak locally
flatpak-install: flatpak-build
    flatpak-builder \
        --user \
        --install \
        --force-clean \
        build-flatpak \
        data/com.system76.CosmicBrowser.json

# Run via Flatpak
flatpak-run:
    flatpak run com.system76.CosmicBrowser

# ── Cleanup ───────────────────────────────────────────────────────────────────

# Remove build artifacts
clean:
    cargo clean
    rm -rf build-flatpak .flatpak-builder

# Remove generated data files
clean-all: clean
    rm -rf ~/.var/app/com.system76.CosmicBrowser
