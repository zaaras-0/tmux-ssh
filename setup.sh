#!/usr/bin/env bash

# Culori pentru output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}🚀 Pornire Setup: tmux-bw-ssh${NC}"

# 1. Asigurare PATH pentru Cargo
if [[ ":$PATH:" != *":$HOME/.cargo/bin:"* ]]; then
    echo -e "${YELLOW}Adding ~/.cargo/bin to PATH...${NC}"
    echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
    echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
    export PATH="$HOME/.cargo/bin:$PATH"
fi

# 2. Verificare Dependențe Sistem
echo -e "${BLUE}📦 Verificare dependențe sistem...${NC}"
if ! command -v gcc &> /dev/null; then
    echo -e "${YELLOW}Installing build-essential...${NC}"
    sudo apt-get update && sudo apt-get install -y build-essential
fi

# 3. Instalare/Verificare RBW
if ! command -v rbw &> /dev/null; then
    echo -e "${YELLOW}Installing rbw (Rust Bitwarden)...${NC}"
    cargo install rbw
else
    echo -e "${GREEN}✅ rbw este deja instalat.${NC}"
fi

# 4. Compilare Proiect
echo -e "${BLUE}🔨 Compilare binar tmux-bw-ssh...${NC}"
cargo build --release

# 5. Permisiuni
chmod +x tmux-bw-ssh.tmux scripts/insert.sh

# 6. Ghid Configurare Vaultwarden
echo -e "
${GREEN}✨ Setup de bază finalizat!${NC}"
echo -e "${YELLOW}Te rog să rulezi următoarele comenzi pentru a te conecta la Vaultwarden:${NC}"
echo -e "1. ${BLUE}rbw config set base_url${NC} https://vault.domeniul-tau.ro"
echo -e "2. ${BLUE}rbw config set email${NC} adresa@ta.com"
echo -e "3. ${BLUE}rbw login${NC}"
echo -e "4. ${BLUE}rbw unlock${NC}"

echo -e "
${BLUE}Pentru a activa scurtăturile în Tmux, rulează:${NC}"
echo -e "${GREEN}/home/zaaras/tmux-ssh/tmux-bw-ssh.tmux${NC}"
echo -e "Sau adaugă în ~/.tmux.conf: ${YELLOW}run-shell "/home/zaaras/tmux-ssh/tmux-bw-ssh.tmux"${NC}"
