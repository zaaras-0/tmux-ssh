#!/usr/bin/env bash

set -e

# --- Configuration ---
REPO="zaaras-0/tmux-ssh"
BINARY_NAME="zbw"
INSTALL_DIR="$HOME/.local/bin"
TMUX_CONF="$HOME/.tmux.conf"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}🚀 Starting zbw Installation...${NC}"

# --- Setup Directories ---
mkdir -p "$INSTALL_DIR"

# --- Add to PATH ---
SHELL_RC_FILES=("$HOME/.bashrc" "$HOME/.zshrc")
PATH_ADDED=false

for RC_FILE in "${SHELL_RC_FILES[@]}"; do
    if [ -f "$RC_FILE" ]; then
        if ! grep -q "$INSTALL_DIR" "$RC_FILE"; then
            echo -e "${YELLOW}Adding $INSTALL_DIR to PATH in $RC_FILE...${NC}"
            echo -e "\n# --- zbw (Bitwarden SSH) PATH ---\nexport PATH=\"\$PATH:$INSTALL_DIR\"" >> "$RC_FILE"
            PATH_ADDED=true
        fi
    fi
done

if [ "$PATH_ADDED" = true ]; then
    echo -e "${YELLOW}⚠️  PATH updated! Please restart your terminal or run: ${BLUE}source ~/.bashrc${NC} (or .zshrc)${NC}"
fi

# --- Install Binary ---
URL=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" \
    | grep "browser_download_url" \
    | cut -d '"' -f 4)

if [ -z "$URL" ]; then
    echo "❌ Error: Could not find a Linux x86_64 binary in the latest release."
    exit 1
fi

# 3. Download and make executable
echo "📥 Downloading: $URL"
curl -L "$URL" -o "$INSTALL_DIR/$BINARY_NAME"
chmod +x "$INSTALL_DIR/$BINARY_NAME"

# --- Tmux Configuration ---
echo -e "${YELLOW}Updating .tmux.conf...${NC}"
# Curățăm vechile configurări pentru a evita dublurile (zbw)
sed -i '/# --- zbw/,+12d' "$TMUX_CONF" 2>/dev/null || true

cat >> "$TMUX_CONF" << EOF

# --- zbw (Bitwarden SSH) configuration ---
bind-key s display-popup -E -w 95% -h 85% "$INSTALL_DIR/$BINARY_NAME"
bind-key S display-popup -E -w 95% -h 85% "$INSTALL_DIR/$BINARY_NAME"
bind-key g display-popup -E -w 95% -h 85% "$INSTALL_DIR/$BINARY_NAME snippets"
bind-key G display-popup -E -w 95% -h 85% "$INSTALL_DIR/$BINARY_NAME snippets"
bind-key "/" command-prompt -p "Search Vault:" "display-popup -E -w 95% -h 85% '$INSTALL_DIR/$BINARY_NAME search \"%%\"'"
bind-key -n C-p run-shell "$INSTALL_DIR/$BINARY_NAME pass"
bind-key p run-shell "$INSTALL_DIR/$BINARY_NAME pass"
EOF

# --- Cleanup ---
if [ -d "target" ]; then
    echo -e "${YELLOW}Cleaning up build files...${NC}"
    rm -rf "target"
fi

# --- Final Steps ---
echo -e "\n${GREEN}✅ Installation Complete!${NC}"
echo -e "1. Run ${YELLOW}$BINARY_NAME config${NC} if needed."
echo -e "2. Reload tmux: ${YELLOW}tmux source-file ~/.tmux.conf${NC}"
