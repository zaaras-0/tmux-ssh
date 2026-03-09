#!/bin/bash

# Culori pentru output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Starting tmux-ssh environment setup...${NC}"

echo -e "Checking system updates..."
sudo apt update -y && sudo apt install -y curl git build-essential

# 2. Install/Update Tmux
if ! command -v tmux &> /dev/null; then
    echo -e "${GREEN}Installing Tmux...${NC}"
    sudo apt install -y tmux
else
    echo -e "Tmux is already installed."
fi

# 3. Install/Update Tmuxinator (Ruby based)
if ! command -v tmuxinator &> /dev/null; then
    echo -e "${GREEN}Installing Tmuxinator...${NC}"
    sudo apt install -y tmuxinator
else
    echo -e "Tmuxinator is already installed."
fi

# 4. Install/Update RBW
if ! command -v rbw &> /dev/null; then
    echo -e "${GREEN}Installing rbw...${NC}"

    VERSION="1.15.0"
    ARCH="amd64"

    echo "Downloading rbw v$VERSION..."
    curl -LO "https://github.com/doy/rbw/releases/download/$VERSION/rbw_${VERSION}_linux_${ARCH}.tar.gz"

    echo "Extracting..."
    tar -xzf "rbw_${VERSION}_linux_${ARCH}.tar.gz"

    sudo mv rbw rbw-agent /usr/local/bin/
    rm "rbw_${VERSION}_linux_${ARCH}.tar.gz"

    echo "rbw installation complete. Version:"
    rbw --version
else
    echo -e "rbw is already installed."
fi

# 5. Download tmux-ssh (Your Rust Binary)
echo -e "${BLUE}Downloading latest tmux-ssh release...${NC}"
# Aici vom folosi GitHub API pentru a lua ultimul release
# Exemplu de structură:
mkdir -p ~/.local/bin
curl -L https://github.com/zaaras-0/tmux-ssh/archive/refs/tags/latest.zip -o ~/.local/bin/tmux-ssh
chmod +x ~/.local/bin/tmux-ssh

# 6. Setup tmux.conf
echo -e "Configuring .tmux.conf..."
if [ -f ~/.tmux.conf ]; then
    mv ~/.tmux.conf ~/.tmux.conf.bak
    echo "Backup created at ~/.tmux.conf.bak"
fi
# Luăm config-ul direct din repo-ul tău public
curl -s https://raw.githubusercontent.com/zaaras-0/tmux-ssh/refs/heads/main/.tmux.conf -o ~/.tmux.conf

echo -e "${GREEN}Setup complete! Restart tmux to apply changes.${NC}"
