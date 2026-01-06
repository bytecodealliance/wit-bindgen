#!/bin/sh
set -e
cd $HOME

# Wasmtime: Used as default test runner.
curl https://wasmtime.dev/install.sh -sSf | bash

# Rust: Install additional targets not present in the docker base image.
rustup target add wasm32-wasip2

# C/C++
WASI_SDK_PATH="$HOME/wasi-sdk-25.0"
mkdir -p "$WASI_SDK_PATH"
curl -L https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-25/wasi-sdk-25.0-x86_64-linux.tar.gz | tar -xz -C "$WASI_SDK_PATH" --strip-components=1
echo "export WASI_SDK_PATH=$WASI_SDK_PATH" >> ~/.bashrc
echo "alias clang=$WASI_SDK_PATH/bin/clang" >> ~/.bashrc
echo "alias clang++=$WASI_SDK_PATH/bin/clang++" >> ~/.bashrc

# .NET
DOTNET_PATH="$HOME/.dotnet"
mkdir -p "$DOTNET_PATH"
curl -sSL https://dot.net/v1/dotnet-install.sh -o $DOTNET_PATH/dotnet-install.sh
chmod +x $DOTNET_PATH/dotnet-install.sh
$DOTNET_PATH/dotnet-install.sh --channel 9.0 --install-dir $DOTNET_PATH
echo "export PATH=$DOTNET_PATH:\$PATH" >> ~/.bashrc

# Moonbit
curl -fsSL https://cli.moonbitlang.com/install/unix.sh | bash
echo 'export PATH="$HOME/.moon/bin:$PATH"' >> ~/.bashrc

# Go
curl -OL https://go.dev/dl/go1.25.5.linux-amd64.tar.gz
tar xf go1.25.5.linux-amd64.tar.gz
echo "export PATH=$HOME/go1.25.5.linux-amd64/bin:\$PATH" >> ~/.bashrc
