#!/bin/bash

PLATFORM="$(uname -s)"
ARCH="$(uname -m)"
BASE_URL="https://fast-down-update.s121.top/cli/download/latest"
INSTALL_DIR="$HOME/.local/bin"
BIN_NAME="fd"
DOWNLOAD_URL="${BASE_URL}/${PLATFORM}/${ARCH}"

TMP_FILE=$(mktemp)
trap 'rm -f "$TMP_FILE"' EXIT INT TERM

echo "✨ Downloading $DOWNLOAD_URL"

if command -v curl >/dev/null 2>&1; then
    HTTP_STATUS=$(curl -L# --retry 3 --retry-delay 2 -w "%{http_code}" -o "$TMP_FILE" "$DOWNLOAD_URL")
    CURL_RET=$?
    if [ $CURL_RET -ne 0 ]; then
        echo "❌ Error: Curl command failed (Exit Code: $CURL_RET)"
        exit 1
    fi
    if [ "$HTTP_STATUS" != "200" ]; then
        if [ -s "$TMP_FILE" ]; then
            SERVER_MSG=$(cat "$TMP_FILE")
            echo "❌ Error: $SERVER_MSG (Platform: $PLATFORM, Arch: $ARCH, HTTP Status: $HTTP_STATUS)"
        else
            echo "❌ Error: Network request failed (Platform: $PLATFORM, Arch: $ARCH, HTTP Status: $HTTP_STATUS)"
        fi
        exit 1
    fi
elif command -v wget >/dev/null 2>&1; then
    wget -q --show-progress --content-on-error -O "$TMP_FILE" "$DOWNLOAD_URL"
    WGET_STATUS=$?
    if [ $WGET_STATUS -ne 0 ]; then
        if [ -s "$TMP_FILE" ]; then
            SERVER_MSG=$(cat "$TMP_FILE")
            echo "❌ Error: $SERVER_MSG (Platform: $PLATFORM, Arch: $ARCH)"
        else
            echo "❌ Error: Network request failed (Platform: $PLATFORM, Arch: $ARCH)"
        fi
        exit 1
    fi
else
    echo "❌ Error: Neither 'curl' nor 'wget' is installed"
    echo "🔔 Please install curl or wget first, then run this script again"
    exit 1
fi

mkdir -p "$INSTALL_DIR"
mv "$TMP_FILE" "$INSTALL_DIR/$BIN_NAME"
chmod +x "$INSTALL_DIR/$BIN_NAME"
trap - EXIT INT TERM
echo "🎉 Installed to $INSTALL_DIR/$BIN_NAME"

SHELL_RC=""

case ":$PATH:" in
    *":$INSTALL_DIR:"*)
        echo "🔔 $INSTALL_DIR is already in your PATH"
        ;;
    *)
        echo "🔧 Adding $INSTALL_DIR to User PATH"
        if [[ "$SHELL" == *"zsh"* ]]; then
            SHELL_RC="$HOME/.zshrc"
        elif [[ "$SHELL" == *"bash"* ]]; then
            if [ "$PLATFORM" = "Darwin" ]; then
                SHELL_RC="$HOME/.bash_profile"
            else
                SHELL_RC="$HOME/.bashrc"
            fi
        else
            SHELL_RC="$HOME/.profile"
        fi

        echo '' >> "$SHELL_RC"
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$SHELL_RC"

        echo "🔔 Please restart your terminal or run 'source $SHELL_RC' for the changes to take full effect"
        ;;
esac

echo "🚀 Installation complete! You can now run '$BIN_NAME'"
