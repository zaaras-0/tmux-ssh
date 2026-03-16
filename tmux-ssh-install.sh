#!/usr/bin/env bash

set -e

# --- Configuration ---
REPO_URL="https://github.com/zaaras-0/tmux-ssh"
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

# --- Install Binary ---
if [ -d "src" ] && [ -f "Cargo.toml" ]; then
    echo -e "${YELLOW}Compiling zbw from source...${NC}"
    cargo build --release
    cp target/release/$BINARY_NAME "$INSTALL_DIR/"
fi

chmod +x "$INSTALL_DIR/$BINARY_NAME"

# --- Install Helper Scripts ---
echo -e "${YELLOW}Installing helper scripts...${NC}"
cat > "$INSTALL_DIR/tmux-insert-pass" << 'EOF'
#!/usr/bin/env bash
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
# Curățăm vechile configurări pentru a evita dublurile
sed -i '/# --- zbw/,+10d' "$TMUX_CONF" 2>/dev/null || true

cat >> "$TMUX_CONF" << EOF

# --- zbw (Bitwarden SSH) configuration ---
bind-key s new-window -n "selector" "$INSTALL_DIR/$BINARY_NAME"
bind-key S new-window -n "selector" "$INSTALL_DIR/$BINARY_NAME"
bind-key g split-window -v -p 30 "$INSTALL_DIR/$BINARY_NAME snippets"
bind-key G split-window -v -p 30 "$INSTALL_DIR/$BINARY_NAME snippets"
bind-key "/" command-prompt -p "Search Vault:" "split-window -v -p 30 '$INSTALL_DIR/$BINARY_NAME search \"%%\"'"
bind-key -n C-p run-shell "$INSTALL_DIR/tmux-insert-pass"
bind-key p run-shell "$INSTALL_DIR/tmux-insert-pass"
EOF

# --- Final Steps ---
echo -e "\n${GREEN}✅ Installation Complete!${NC}"
echo -e "1. Run ${YELLOW}$BINARY_NAME config${NC} if needed."
echo -e "2. Reload tmux: ${YELLOW}tmux source-file ~/.tmux.conf${NC}"
