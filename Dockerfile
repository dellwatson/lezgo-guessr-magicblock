# Use Ubuntu as base and install Solana toolchain
FROM ubuntu:22.04

# Avoid interactive prompts
ENV DEBIAN_FRONTEND=noninteractive

# Install dependencies
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install Solana CLI
RUN sh -c "$(curl -sSfL https://release.solana.com/v1.18.18/install)"
ENV PATH="/root/.local/share/solana/install/active_release/bin:${PATH}"

# Set working directory
WORKDIR /app

# Copy the entire programs directory
COPY . .

# Install cargo build-sbf
RUN cargo install cargo-build-sbf --force

# Show versions
RUN rustc --version && cargo --version && cargo build-sbf --version

# Build the program
RUN cd magicblock-guessr && cargo build-sbf

# Copy the built program to artifacts directory
RUN mkdir -p artifacts && \
    cp magicblock-guessr/target/deploy/guessr_multiplayer_program.so artifacts/ || \
    cp magicblock-guessr/target/release/libguessr_multiplayer_program.so artifacts/ || \
    echo "No .so file found, checking what was built..." && \
    find magicblock-guessr/target -name "*.so" -o -name "*.dylib" -o -name "*.rlib"

# Show what we built
RUN ls -la artifacts/ || echo "Artifacts directory empty"

# Keep container running for inspection
CMD ["bash"]
