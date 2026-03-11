#!/bin/bash

echo "🚀 Solana Program Build Script - magicblock-guessr-v1"
echo "====================================================="

echo "📋 Current Rust version:"
rustc --version
cargo --version

echo ""
echo "🔧 Trying cargo build-sbf..."

# Try building with cargo build-sbf
if cargo build-sbf 2>&1; then
    echo "✅ Build successful with cargo build-sbf!"
    
    # Copy artifacts if they exist
    if [ -f "target/deploy/guessr_multiplayer_program_v1.so" ]; then
        mkdir -p artifacts
        cp target/deploy/guessr_multiplayer_program_v1.so artifacts/
        echo "✅ Artifact copied to ./artifacts/"
    elif [ -f "target/release/libguessr_multiplayer_program_v1.so" ]; then
        mkdir -p artifacts
        cp target/release/libguessr_multiplayer_program_v1.so artifacts/
        echo "✅ Artifact copied to ./artifacts/"
    else
        echo "⚠️  Build succeeded but no .so file found"
        find target -name "*.so" -o -name "*.dylib" | head -5
    fi
else
    echo "❌ cargo build-sbf failed"
    
    echo ""
    echo "🔧 Trying regular cargo build..."
    
    # Fallback to regular cargo build
    if cargo build --release; then
        echo "✅ Regular build successful!"
        
        if [ -f "target/release/libguessr_multiplayer_program_v1.dylib" ]; then
            mkdir -p artifacts
            cp target/release/libguessr_multiplayer_program_v1.dylib artifacts/guessr_multiplayer_program_v1.so
            echo "✅ Copied dylib as .so file (for testing only) to ./artifacts/"
        fi
    else
        echo "❌ Regular build also failed"
    fi
fi

echo ""
echo "📦 Artifacts in artifacts directory:"
ls -la artifacts/ 2>/dev/null || echo "No artifacts directory"

echo ""
echo "🐳 To try Docker build:"
echo "docker build -t solana-guessr-builder ."
echo "docker run -it --rm -v \$(pwd)/artifacts:/app/artifacts solana-guessr-builder"
