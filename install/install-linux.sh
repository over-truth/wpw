#!/bin/bash
# WPW Password Manager - Linux Installer
# Run: ./install-linux.sh [--extension-id ID] [--uninstall]

set -e

HOST_NAME="com.wpw.host"
INSTALL_DIR="$HOME/.local/share/wpw"
BIN_DIR="$HOME/.local/bin"

# Parse arguments
EXTENSION_ID=""
UNINSTALL=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --extension-id) EXTENSION_ID="$2"; shift 2 ;;
        --uninstall) UNINSTALL=true; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

if [ "$UNINSTALL" = true ]; then
    echo "Uninstalling WPW..."
    
    # Remove NM manifests
    rm -f "$HOME/.config/google-chrome/NativeMessagingHosts/$HOST_NAME.json"
    rm -f "$HOME/.config/microsoft-edge/NativeMessagingHosts/$HOST_NAME.json"
    rm -f "$HOME/.config/chromium/NativeMessagingHosts/$HOST_NAME.json"
    
    # Remove binaries
    rm -f "$BIN_DIR/wpw"
    rm -f "$BIN_DIR/wpw-host"
    rm -rf "$INSTALL_DIR"
    
    echo "WPW uninstalled successfully."
    exit 0
fi

# Installation
echo "Installing WPW Password Manager..."

# Create directories
mkdir -p "$BIN_DIR"
mkdir -p "$INSTALL_DIR"

# Copy binaries
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

if [ -f "$PROJECT_ROOT/target/release/wpw" ]; then
    cp "$PROJECT_ROOT/target/release/wpw" "$BIN_DIR/wpw"
    cp "$PROJECT_ROOT/target/release/wpw-host" "$BIN_DIR/wpw-host"
    chmod +x "$BIN_DIR/wpw" "$BIN_DIR/wpw-host"
    echo "Copied binaries to $BIN_DIR"
else
    echo "Warning: Release binaries not found. Please build first with 'cargo build --release'"
fi

# Check if ~/.local/bin is in PATH
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.bashrc"
    echo "Added ~/.local/bin to PATH in .bashrc"
    
    # Also try .zshrc
    if [ -f "$HOME/.zshrc" ]; then
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.zshrc"
        echo "Added ~/.local/bin to PATH in .zshrc"
    fi
fi

# Generate NM manifest
ALLOWED_ORIGINS="[]"
if [ -n "$EXTENSION_ID" ]; then
    ALLOWED_ORIGINS="[\"chrome-extension://$EXTENSION_ID/\"]"
fi

cat > "$INSTALL_DIR/$HOST_NAME.json" << EOF
{
  "name": "$HOST_NAME",
  "description": "WPW Password Manager Native Host",
  "path": "$BIN_DIR/wpw-host",
  "type": "stdio",
  "allowed_origins": $ALLOWED_ORIGINS
}
EOF

echo "Created NM manifest at $INSTALL_DIR/$HOST_NAME.json"

# Install for Chrome
CHROME_NM_DIR="$HOME/.config/google-chrome/NativeMessagingHosts"
mkdir -p "$CHROME_NM_DIR"
cp "$INSTALL_DIR/$HOST_NAME.json" "$CHROME_NM_DIR/$HOST_NAME.json"
echo "Registered for Google Chrome"

# Install for Edge
EDGE_NM_DIR="$HOME/.config/microsoft-edge/NativeMessagingHosts"
mkdir -p "$EDGE_NM_DIR"
cp "$INSTALL_DIR/$HOST_NAME.json" "$EDGE_NM_DIR/$HOST_NAME.json"
echo "Registered for Microsoft Edge"

# Install for Chromium (if present)
CHROMIUM_NM_DIR="$HOME/.config/chromium/NativeMessagingHosts"
if [ -d "$HOME/.config/chromium" ]; then
    mkdir -p "$CHROMIUM_NM_DIR"
    cp "$INSTALL_DIR/$HOST_NAME.json" "$CHROMIUM_NM_DIR/$HOST_NAME.json"
    echo "Registered for Chromium"
fi

echo ""
echo "Installation complete!"
echo "Please restart your terminal or run: source ~/.bashrc"
echo "Run 'wpw init' to create your vault."
