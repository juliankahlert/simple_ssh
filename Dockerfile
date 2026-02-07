# syntax=docker/dockerfile:1.4
# Dockerfile for simple-ssh-builder
# Builds a container with all tools needed to compile the simple-ssh Rust project
# Usage:
#   docker build -t simple-ssh-builder .
#   docker run -it -v $(pwd):/workspace simple-ssh-builder

FROM debian@sha256:98f4b71de414932439ac6ac690d7060df1f27161073c5036a7553723881bffbe

# Set environment variables
ENV DEBIAN_FRONTEND=noninteractive
ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup
ENV PATH=$CARGO_HOME/bin:$PATH
ENV CLICOLOR_FORCE=1

# Install system dependencies (single layer to reduce image size)
RUN dpkg --add-architecture i386 && \
    apt-get update && apt-get install -y --no-install-recommends \
    # Basic build tools
    build-essential \
    pkg-config \
    cmake \
    git \
    ca-certificates \
    curl \
    wget \
    xz-utils \
    # SSL and crypto libraries (russh dependencies)
    libssl-dev \
    libclang-dev \
    clang \
    # For musl targets
    musl-tools \
    musl-dev \
    # Clean up to reduce image size
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

# Download and install musl cross-compilation toolchains from musl.cc
# These provide the actual musl-based compilers needed for static linking
RUN mkdir -p /usr/local/musl-cross && cd /usr/local/musl-cross && \
    # Download x86_64 musl toolchain
    wget -q https://musl.cc/x86_64-linux-musl-cross.tgz && \
    tar -xzf x86_64-linux-musl-cross.tgz && \
    rm x86_64-linux-musl-cross.tgz && \
    # Download i686 musl toolchain
    wget -q https://musl.cc/i686-linux-musl-cross.tgz && \
    tar -xzf i686-linux-musl-cross.tgz && \
    rm i686-linux-musl-cross.tgz && \
    # Download aarch64 musl toolchain
    wget -q https://musl.cc/aarch64-linux-musl-cross.tgz && \
    tar -xzf aarch64-linux-musl-cross.tgz && \
    rm aarch64-linux-musl-cross.tgz && \
    # Download arm musl toolchain
    wget -q https://musl.cc/arm-linux-musleabihf-cross.tgz && \
    tar -xzf arm-linux-musleabihf-cross.tgz && \
    rm arm-linux-musleabihf-cross.tgz

# Add musl cross-compilers to PATH
ENV PATH="/usr/local/musl-cross/i686-linux-musl-cross/bin:/usr/local/musl-cross/x86_64-linux-musl-cross/bin:/usr/local/musl-cross/aarch64-linux-musl-cross/bin:/usr/local/musl-cross/arm-linux-musleabihf-cross/bin:/usr/bin:${PATH}"

# Create explicit symlinks for musl compilers from tarball triplet paths
RUN for triplet in i686-linux-musl x86_64-linux-musl aarch64-linux-musl arm-linux-musleabihf; do \
        musl_bin="/usr/local/musl-cross/${triplet}-cross/bin"; \
        case "$triplet" in \
            i686-linux-musl) symlink="i686-linux-musl-gcc" ;; \
            x86_64-linux-musl) symlink="x86_64-linux-musl-gcc" ;; \
            aarch64-linux-musl) symlink="aarch64-linux-musl-gcc" ;; \
            arm-linux-musleabihf) symlink="arm-linux-musleabihf-gcc" ;; \
        esac && \
        if [ -f "${musl_bin}/${symlink}" ]; then \
            ln -sf "${musl_bin}/${symlink}" "/usr/local/bin/${symlink}"; \
        fi; \
    done

# Install Rust in a separate layer for better caching
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain stable --profile minimal && \
    rm -rf $CARGO_HOME/registry/cache/*

# Add musl targets for cross-compilation
RUN rustup target add \
    x86_64-unknown-linux-musl \
    i686-unknown-linux-musl \
    aarch64-unknown-linux-musl \
    arm-unknown-linux-musleabihf

# Add linker configurations for cross-compilation
RUN mkdir -p $CARGO_HOME && \
    cat > $CARGO_HOME/config.toml << 'EOF'
[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"
rustflags = ["-C", "target-feature=+crt-static", "-C", "relocation-model=static", "-C", "link-arg=-static"]

[target.i686-unknown-linux-musl]
linker = "i686-linux-musl-gcc"
rustflags = ["-C", "target-feature=+crt-static", "-C", "relocation-model=static", "-C", "link-arg=-static"]

[target.aarch64-unknown-linux-musl]
linker = "aarch64-linux-musl-gcc"
rustflags = ["-C", "target-feature=+crt-static", "-C", "relocation-model=static", "-C", "link-arg=-static"]

[target.arm-unknown-linux-musleabihf]
linker = "arm-linux-musleabihf-gcc"
rustflags = ["-C", "target-feature=+crt-static", "-C", "relocation-model=static", "-C", "link-arg=-static"]
EOF

# Install cargo-audit for security checks (version 0.22.1 pinned for reproducibility)
RUN cargo install cargo-audit --version 0.22.1 --locked && \
    rm -rf $CARGO_HOME/registry/cache/*

# Install just command runner (version 1.46.0 pinned for reproducibility)
RUN cargo install just --version 1.46.0 --locked && \
    rm -rf $CARGO_HOME/registry/cache/*

# Create workspace directory
WORKDIR /workspace

# Make cargo/rustup directories world-writable for running as any user
RUN chmod -R 777 $CARGO_HOME $RUSTUP_HOME

# Verify installation
RUN echo "=== simple-ssh-builder environment ===" && \
    rustc --version && \
    cargo --version && \
    echo "Installed targets:" && \
    rustup target list --installed && \
    echo "=== Verifying cross-compiler toolchains ===" && \
    for target in $(rustup target list --installed); do \
        case "$target" in \
            x86_64-unknown-linux-musl) linker="x86_64-linux-musl-gcc" ;; \
            i686-unknown-linux-musl) linker="i686-linux-musl-gcc" ;; \
            aarch64-unknown-linux-musl) linker="aarch64-linux-musl-gcc" ;; \
            arm-unknown-linux-musleabihf) linker="arm-linux-musleabihf-gcc" ;; \
            *) echo "Skipping unknown target: $target"; continue ;; \
        esac && \
        if command -v "$linker" >/dev/null 2>&1; then \
            echo "Found $linker: $($linker --version | head -n1)" || true; \
        else \
            echo "ERROR: Missing cross-compiler for $target: $linker" && exit 1; \
        fi || exit 1; \
    done

# Set default command
CMD ["/bin/bash"]
