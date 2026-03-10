#!/bin/bash

echo "🚀 Solana Program Build Script"
echo "=============================="

# Function to show usage
show_usage() {
    echo "Usage: $0 [version]"
    echo ""
    echo "Versions:"
    echo "  magicblock-guessr  (default) - Build the current version"
    echo "  magicblock-guessr-v1         - Build the v1 version"
    echo ""
    echo "Examples:"
    echo "  $0                # Build magicblock-guessr"
    echo "  $0 magicblock-guessr     # Build magicblock-guessr"
    echo "  $0 magicblock-guessr-v1  # Build magicblock-guessr-v1"
}

# Determine which version to build
VERSION=${1:-magicblock-guessr}

case $VERSION in
    "magicblock-guessr")
        echo "� Building magicblock-guessr..."
        cd magicblock-guessr
        ./build.sh
        ;;
    "magicblock-guessr-v1")
        echo "📦 Building magicblock-guessr-v1..."
        cd magicblock-guessr-v1
        ./build.sh
        ;;
    "help"|"-h"|"--help")
        show_usage
        exit 0
        *)
        echo "❌ Unknown version: $VERSION"
        echo ""
        show_usage
        exit 1
        ;;
esac
