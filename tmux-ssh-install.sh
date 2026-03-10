#!/usr/bin/env bash

set -e

# --- Configuration ---
REPO_URL="https://github.com/zaaras-0/tmux-ssh"
BINARY_NAME="tmux-bw-ssh"
INSTALL_DIR="$HOME/.local/bin"
SCRIPTS_DIR="$HOME/.local/share/tmux-ssh"
TMUX_CONF="$HOME/.tmux.conf"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}🚀 Starting tmux-bw-ssh Installation...${NC}"

# --- Prerequisites ---
echo -e "${YELLOW}Checking dependencies...${NC}"

# Check for tmux
if ! command -v tmux &> /dev/null; then
    echo "Installing tmux..."
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        sudo apt update && sudo apt install -y tmux
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        brew install tmux
    fi
fi

# Check for rbw
if ! command -v rbw &> /dev/null; then
    echo -e "${YELLOW}Installing rbw (Bitwarden CLI)...${NC}"
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        # Simple download for Linux amd64 as example
        VERSION=$(curl -s https://api.github.com/repos/doy/rbw/releases/latest | grep tag_name | cut -d '"' -f 4)
        curl -LO "https://github.com/doy/rbw/releases/download/${VERSION}/rbw_${VERSION}_linux_amd64.tar.gz"
        tar -xzf "rbw_${VERSION}_linux_amd64.tar.gz"
        sudo mv rbw rbw-agent /usr/local/bin/
        rm "rbw_${VERSION}_linux_amd64.tar.gz"
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        brew install rbw
    fi
fi

# --- Setup Directories ---
mkdir -p "$INSTALL_DIR"
mkdir -p "$SCRIPTS_DIR"

# --- Install Binary ---
# Note: For development, we'll compile it. In a real 'curl | sh' scenario, 
# we would download the pre-compiled binary from GitHub Releases.
if [ -d "src" ] && [ -f "Cargo.toml" ]; then
    echo -e "${YELLOW}Compiling tmux-bw-ssh from source...${NC}"
    cargo build --release
    cp target/release/$BINARY_NAME "$INSTALL_DIR/"
else
    echo -e "${YELLOW}Downloading pre-compiled binary...${NC}"
    # Placeholder for actual download logic
    curl -L "$REPO_URL/releases/latest/download/$BINARY_NAME" -o "$INSTALL_DIR/$BINARY_NAME"
    echo "No source found, and download not yet implemented for dev."
    # For now, let's assume we are in the repo if we run this
fi

chmod +x "$INSTALL_DIR/$BINARY_NAME"

# --- Install Scripts ---
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
cp ./.tmux.conf "$TMUX_CONF"
echo -e "${YELLOW}Updating .tmux.conf...${NC}"
if ! grep -q "tmux-bw-ssh" "$TMUX_CONF" 2>/dev/null; then
    cat >> "$TMUX_CONF" << EOF

# --- tmux-bw-ssh configuration ---
bind-key "S" new-window -n "selector" "$INSTALL_DIR/$BINARY_NAME"
bind-key "G" display-popup -E -w 80% -h 70% "$INSTALL_DIR/$BINARY_NAME --snippets"
bind-key -n C-p run-shell "$INSTALL_DIR/tmux-insert-pass"
bind-key "p" run-shell "$INSTALL_DIR/tmux-insert-pass"
EOF
    echo -e "${GREEN}Configuration added to $TMUX_CONF${NC}"
else
    echo -e "Configuration already exists in $TMUX_CONF"
fi

# --- Final Steps ---
echo -e "\n${GREEN}✅ Installation Complete!${NC}"
echo -e "1. Run ${YELLOW}rbw config set email your@email.com${NC} if you haven't already."
echo -e "2. Run ${YELLOW}rbw login${NC} to authenticate."
echo -e "3. Reload tmux with ${YELLOW}tmux source-file ~/.tmux.conf${NC} or restart it."
echo -e "\nPress ${BLUE}Prefix + S${NC} for Servers or ${BLUE}Prefix + G${NC} for Snippets."
