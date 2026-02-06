# Justfile - Build commands for simple-ssh project
# Usage: just <command>
# Install just: https://github.com/casey/just
#
# Container engine can be set via CONTAINER_ENGINE environment variable.
# If not set, will auto-detect (prefers podman if available, falls back to docker)
# Example: CONTAINER_ENGINE=podman just build-image

# Auto-detect container engine: use CONTAINER_ENGINE if set, otherwise prefer podman
cmd := if env_var_or_default("CONTAINER_ENGINE", "") != "" {
    env_var("CONTAINER_ENGINE")
} else if `which podman 2>/dev/null || echo ""` != "" {
    "podman"
} else {
    "docker"
}

# Default build target
target := "development"

# Default target - show available commands
default:
    @just --list

full-check:
    cargo test
    cargo clippy
    cargo build
    coderabbit review --base main --prompt-only -t uncommitted

# Build the container image with all build tools
build-image:
    @printf "\033[38;2;245;194;122m[build-image]\033[0m "
    @printf "Building simple-ssh-builder image with {{cmd}}...\n"
    {{cmd}} build -t simple-ssh-builder .

# Run the build container interactively
shell:
    @printf "\033[38;2;157;220;210m[shell]\033[0m "
    @printf "Starting interactive shell in build container...\n"
    {{cmd}} run -it -v $(pwd):/workspace simple-ssh-builder

# Build for the native target
development:
    @printf "\033[38;2;210;210;210m[development]\033[0m "
    @printf "Building for native target (debug)...\n"
    cargo build

# Build for native target in release mode
release:
    @printf "\033[38;2;210;210;210m[release]\033[0m "
    @printf "Building for native target (release)...\n"
    cargo build --release

# Build for x86_64 musl target
musl-amd64:
    @printf "\033[38;2;210;210;210m[musl-amd64]\033[0m "
    @printf "Building for x86_64-unknown-linux-musl...\n"
    cargo build --release --target x86_64-unknown-linux-musl

# Build for i686 musl target
musl-i686:
    @printf "\033[38;2;210;210;210m[musl-i686]\033[0m "
    @printf "Building for i686-unknown-linux-musl...\n"
    cargo build --release --target i686-unknown-linux-musl

# Build for aarch64 musl target
musl-aarch64:
    @printf "\033[38;2;210;210;210m[musl-aarch64]\033[0m "
    @printf "Building for aarch64-unknown-linux-musl...\n"
    cargo build --release --target aarch64-unknown-linux-musl

# Build for arm musl target
musl-arm:
    @printf "\033[38;2;210;210;210m[musl-arm]\033[0m "
    @printf "Building for arm-unknown-linux-musleabihf...\n"
    cargo build --release --target arm-unknown-linux-musleabihf

# Build for all musl targets
musl-all: musl-amd64 musl-i686 musl-aarch64 musl-arm

# Run all builds (native + all musl targets)
build-all: development release musl-all

# Run tests
test:
    @printf "\033[38;2;157;210;157m[test]\033[0m "
    @printf "Running tests...\n"
    cargo test

# Check code formatting
format-check:
    @printf "\033[38;2;210;210;210m[format-check]\033[0m "
    @printf "Checking code formatting...\n"
    cargo fmt -- --check

# Auto-format code
format:
    @printf "\033[38;2;210;210;210m[format]\033[0m "
    @printf "Formatting code...\n"
    cargo fmt

# Run clippy lints
clippy:
    @printf "\033[38;2;210;210;210m[clippy]\033[0m "
    @printf "Running clippy...\n"
    cargo clippy

# Run clippy with strict warnings
clippy-strict:
    @printf "\033[38;2;210;210;210m[clippy-strict]\033[0m "
    @printf "Running clippy with strict warnings...\n"
    cargo clippy -- -D warnings

# Check for security vulnerabilities
audit:
    @printf "\033[38;2;210;210;210m[audit]\033[0m "
    @printf "Checking for security vulnerabilities...\n"
    cargo audit

# Run all checks (format, clippy, test, audit)
check: format-check clippy test audit

# Clean build artifacts
clean:
    @printf "\033[38;2;210;210;210m[clean]\033[0m "
    @printf "Cleaning build artifacts...\n"
    cargo clean

# Build using container (useful when you don't have Rust installed locally)
container-build target="development":
    @printf "\033[38;2;157;180;245m[container-build]\033[0m "
    @printf "Building {{target}} in container...\n"
    {{cmd}} run --rm -v $(pwd):/workspace:Z -w /workspace -e CLICOLOR_FORCE=1 simple-ssh-builder just {{target}}

# Build all musl targets using container
container-musl-all:
    @printf "\033[38;2;157;180;245m[container-musl-all]\033[0m "
    @printf "Building all musl targets in container...\n"
    {{cmd}} run --rm -v $(pwd):/workspace:Z -w /workspace -e CLICOLOR_FORCE=1 simple-ssh-builder just musl-all

# Run tests in container
container-test:
    @printf "\033[38;2;157;180;245m[container-test]\033[0m "
    @printf "Running tests in container...\n"
    {{cmd}} run --rm -v $(pwd):/workspace:Z -w /workspace -e CLICOLOR_FORCE=1 simple-ssh-builder just test

# Show help
help:
    @printf "\033[38;2;157;193;255msimple-ssh build commands\033[0m\n"
    @printf "\n"
    @printf "\033[38;2;134;180;210mContainer engine:\033[0m {{cmd}}\n"
    @printf "  Set \033[38;2;245;210;157mCONTAINER_ENGINE=docker\033[0m or \033[38;2;245;210;157mCONTAINER_ENGINE=podman\033[0m to override\n"
    @printf "\n"
    @printf "\033[1m\033[38;2;190;150;245mRequired before committing:\033[0m\n"
    @printf "  \033[38;2;157;210;157mjust full-check\033[0m         - Run tests, clippy, build, and coderabbit review\n"
    @printf "\n"
    @printf "\033[1m\033[38;2;157;210;180mSetup:\033[0m\n"
    @printf "  \033[38;2;245;194;122mjust build-image\033[0m        - Build the simple-ssh-builder container image\n"
    @printf "\n"
    @printf "\033[1m\033[38;2;157;180;245mLocal builds (requires Rust installed):\033[0m\n"
    @printf "  \033[38;2;210;210;210mjust development\033[0m        - Build for native target (debug)\n"
    @printf "  \033[38;2;210;210;210mjust release\033[0m            - Build for native target (release)\n"
    @printf "  \033[38;2;210;210;210mjust musl-amd64\033[0m         - Build for x86_64 musl\n"
    @printf "  \033[38;2;210;210;210mjust musl-i686\033[0m          - Build for i686 musl\n"
    @printf "  \033[38;2;210;210;210mjust musl-aarch64\033[0m       - Build for aarch64 musl\n"
    @printf "  \033[38;2;210;210;210mjust musl-arm\033[0m           - Build for arm musl\n"
    @printf "  \033[38;2;210;210;210mjust musl-all\033[0m           - Build all musl targets\n"
    @printf "  \033[38;2;210;210;210mjust build-all\033[0m          - Build all targets\n"
    @printf "\n"
    @printf "\033[1m\033[38;2;157;220;157mQuality checks:\033[0m\n"
    @printf "  \033[38;2;210;210;210mjust test\033[0m               - Run tests\n"
    @printf "  \033[38;2;210;210;210mjust format\033[0m             - Format code\n"
    @printf "  \033[38;2;210;210;210mjust format-check\033[0m       - Check formatting\n"
    @printf "  \033[38;2;210;210;210mjust clippy\033[0m             - Run clippy\n"
    @printf "  \033[38;2;210;210;210mjust clippy-strict\033[0m      - Run clippy with strict warnings\n"
    @printf "  \033[38;2;210;210;210mjust audit\033[0m              - Run security audit\n"
    @printf "  \033[38;2;210;210;210mjust check\033[0m              - Run all checks\n"
    @printf "\n"
    @printf "\033[1m\033[38;2;157;180;245mContainer builds (no local Rust needed):\033[0m\n"
    @printf "  \033[38;2;210;210;210mjust container-build\033[0m    - Run a build in container\n"
    @printf "  \033[38;2;210;210;210mjust container-musl-all\033[0m - Build all musl targets in container\n"
    @printf "  \033[38;2;210;210;210mjust container-test\033[0m     - Run tests in container\n"
    @printf "  \033[38;2;157;220;210mjust shell\033[0m              - Open interactive shell in build container\n"
