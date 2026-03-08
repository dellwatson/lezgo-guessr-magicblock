# Solana Program Docker Build

## Problem

The current `cargo build-sbf` (v1.84.0) doesn't support Rust edition2024, which is required by the `constant_time_eq v0.4.2` dependency.

## Solutions

### 1. Docker Build (Recommended)

```bash
cd @programs
docker build -t solana-guessr-builder .
docker run -it --rm -v $(pwd)/artifacts:/app/artifacts solana-guessr-builder
```

### 2. Manual Toolchain Update

```bash
# Install latest Solana CLI with newer cargo-build-sbf
curl -sSfL https://release.solana.com/v1.18.18/install | sh
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"

# Install latest cargo-build-sbf
cargo install cargo-build-sbf --force

# Try building
cd magicblock-guessr
cargo build-sbf
```

### 3. Remove Problematic Dependencies

Use `Cargo-no-anchor.toml` to remove Anchor dependencies:

```bash
cd magicblock-guessr
cp Cargo-no-anchor.toml Cargo.toml
cargo build-sbf
```

## Current Status

- Rust edition: 2021
- Cargo version: 1.96.0-nightly
- cargo-build-sbf version: 4.0.0 (uses embedded Cargo 1.84.0)
- Issue: `constant_time_eq v0.4.2` requires edition2024

## Files Created

- `Dockerfile` - Ubuntu-based build environment
- `docker-compose.yml` - Easier container management
- `Cargo-no-anchor.toml` - Dependencies without Anchor
- `.gitignore` - Ignore build artifacts

## Next Steps

1. Try Docker build first
2. If that fails, we may need to:
   - Wait for newer cargo-build-sbf with edition2024 support
   - Downgrade constant_time_eq dependency
   - Use alternative constant-time comparison crate
