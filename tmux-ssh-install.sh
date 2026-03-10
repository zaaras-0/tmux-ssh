#!/usr/bin/env bash

set -e

# --- Configuration ---
REPO_URL="https://github.com/zaaras-0/tmux-ssh"
RAW_REPO="https://raw.githubusercontent.com/zaaras-0/tmux-ssh"
BINARY_NAME="zbw"
INSTALL_DIR="$HOME/.local/bin"
TMUX_CONF="$HOME/.tmux.conf"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}🚀 Starting zbw (tmux-bw-ssh) Installation...${NC}"

# --- Setup Directories ---
mkdir -p "$INSTALL_DIR"

# --- Install Binary ---
if [ -d "src" ] && [ -f "Cargo.toml" ]; then
    echo -e "${YELLOW}Compiling zbw from source...${NC}"
    cargo build --release
    cp target/release/$BINARY_NAME "$INSTALL_DIR/"
else
    echo -e "${RED}Error: Source directory not found. Please run this script from the repository root.${NC}"
    exit 1
fi

chmod +x "$INSTALL_DIR/$BINARY_NAME"

# --- Install Helper Scripts ---
echo -e "${YELLOW}Installing helper scripts...${NC}"
cat > "$INSTALL_DIR/tmux-insert-pass" << 'EOF'
#!/usr/bin/env bash
# Injectează parola salvată în buffer-ul pane-ului curent
PASS=$(tmux show-options -pv @server_pass)
if [ -n "$PASS" ]; then
    tmux send-keys "$PASS" Enter
    tmux display-message "Password injected securely 🔐"
else
    tmux display-message "❌ No password found for this pane (@server_pass)"
fi
EOF
chmod +x "$INSTALL_DIR/tmux-insert-pass"

# --- Tmux Configuration ---
echo -e "${YELLOW}Updating .tmux.conf...${NC}"
if ! grep -q "$BINARY_NAME" "$TMUX_CONF" 2>/dev/null; then
    cat >> "$TMUX_CONF" << EOF

# --- zbw (Bitwarden SSH) configuration ---
bind-key "S" new-window -n "selector" "$INSTALL_DIR/$BINARY_NAME list"
bind-key "G" new-window -n "snippets" "$INSTALL_DIR/$BINARY_NAME snippets"
bind-key "/" command-prompt -p "Search Vault:" "new-window -n 'search' '$INSTALL_DIR/$BINARY_NAME search \"%%\"'"
bind-key -n C-p run-shell "$INSTALL_DIR/tmux-insert-pass"
bind-key "p" run-shell "$INSTALL_DIR/tmux-insert-pass"
EOF
    echo -e "${GREEN}Keybindings added to $TMUX_CONF${NC}"
else
    echo -e "Configuration already exists in $TMUX_CONF"
fi

# --- Final Steps ---
echo -e "\n${GREEN}✅ Installation Complete!${NC}"
echo -e "1. Run ${YELLOW}$BINARY_NAME config${NC} to set up your vault."
echo -e "2. Reload tmux with ${YELLOW}tmux source-file ~/.tmux.conf${NC}"
echo -e "\nKeybindings:"
echo -e "  - ${BLUE}Prefix + S${NC}: List Servers"
echo -e "  - ${BLUE}Prefix + G${NC}: List Snippets"
echo -e "  - ${BLUE}Prefix + /${NC}: Search Vault"
echo -e "  - ${BLUE}Ctrl + P${NC} (or Prefix+p): Inject Password"
