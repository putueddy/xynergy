#!/bin/bash

# Build script for Xynergy frontend WASM

set -e

echo "Building Xynergy frontend..."

# Build Tailwind CSS
echo "Building CSS..."
cd src/frontend
npm run build
cd ../..

# Build WASM (library only, no binary)
echo "Building WASM library..."
cargo build --package xynergy-frontend --target wasm32-unknown-unknown --release --lib

# Create pkg directory
mkdir -p target/site/pkg

# Bindgen the WASM module
echo "Running wasm-bindgen..."
wasm-bindgen target/wasm32-unknown-unknown/release/xynergy_frontend.wasm \
    --out-dir target/site/pkg \
    --target web \
    --no-typescript

echo "Build complete!"
echo "Files in target/site/pkg:"
ls -la target/site/pkg/
