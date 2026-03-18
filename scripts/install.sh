#!/bin/bash

set -e

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux*)  OS_NAME="linux" ;;
    Darwin*) OS_NAME="macos" ;;
    *)       echo "❌ Error: Unsupported OS: $OS. This script only supports Linux and macOS."; exit 1 ;;
esac

case "$ARCH" in
    x86_64|amd64)        ARCH_NAME="x86_64" ;;
    i386|i686|x86)       ARCH_NAME="i686" ;;
    aarch64|arm64|armv8) ARCH_NAME="aarch64" ;;
    *)                   echo "❌ Error: Unsupported Architecture: $ARCH"; exit 1 ;;
esac

if [ "$OS_NAME" = "macos" ]; then
    if [ "$ARCH_NAME" != "aarch64" ] && [ "$ARCH_NAME" != "x86_64" ]; then
        echo "❌ Error: Unsupported architecture for macOS: $ARCH_NAME. Only aarch64 and x86_64 are supported."
        exit 1
    fi
fi

BASE_URL="https://fast-down-update.s121.top/cli/download/latest"
DOWNLOAD_URL="${BASE_URL}/${OS_NAME}/${ARCH_NAME}"

INSTALL_DIR="$HOME/.local/bin"
BIN_NAME="fast-down"
TMP_FILE=$(mktemp)

# 5. Download the binary
echo "Downloading $DOWNLOAD_URL ..."
if command -v curl >/dev/null 2>&1; then
    curl -# -L -o "$TMP_FILE" "$DOWNLOAD_URL"
elif command -v wget >/dev/null 2>&1; then
    wget -q --show-progress -O "$TMP_FILE" "$DOWNLOAD_URL"
else
    echo "❌ Error: curl or wget is required to download the file."
    rm -f "$TMP_FILE"
    exit 1
fi

chmod +x "$TMP_FILE"
mkdir -p "$INSTALL_DIR"
mv "$TMP_FILE" "$INSTALL_DIR/$BIN_NAME"

echo "🎉 Installed to $INSTALL_DIR/$BIN_NAME"

if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo "⚠️ Note: You may need to add $INSTALL_DIR to your PATH in your ~/.bashrc or ~/.zshrc."
else
    echo "🚀 You can now run '$BIN_NAME' from your terminal!"
fi
