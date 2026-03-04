#!/usr/bin/env bash

# Configurare
INSTALL_DIR="$HOME/.local/bin"
PROJECT_DIR="$HOME/tmux-ssh"
BINARY_URL="URL_CATRE_ZIP_BINAR" # Placeholder pentru link-ul tău de download

# Culori
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}🌟 Incepere instalare rapida tmux-bw-ssh${NC}"

# 1. Verificare/Instalare TMUX
if ! command -v tmux &> /dev/null; then
    echo -e "${YELLOW}Instalare tmux...${NC}"
    sudo apt-get update && sudo apt-get install -y tmux
else
    echo -e "${GREEN}✅ Tmux este instalat.${NC}"
fi

# 2. Verificare/Instalare RBW
if ! command -v rbw &> /dev/null; then
    echo -e "${YELLOW}RBW nu a fost gasit. Se recomanda instalarea prin cargo sau manager de pachete.${NC}"
    # Putem adauga aici un link catre binarul rbw de pe github daca dorim
else
    echo -e "${GREEN}✅ RBW este instalat.${NC}"
fi

# 3. Download si Instalare Binar
mkdir -p "$INSTALL_DIR"
if [ ! -f "$PROJECT_DIR/target/release/tmux-bw-ssh" ]; then
    echo -e "${YELLOW}Descarcare binar...${NC}"
    # Exemplu de logica pentru zip:
    # curl -L "$BINARY_URL" -o tssh.zip && unzip tssh.zip -d "$INSTALL_DIR" && rm tssh.zip
    # Pentru moment, daca suntem pe device-ul unde s-a compilat, il copiem pur si simplu
    cp "$PROJECT_DIR/target/release/tmux-bw-ssh" "$INSTALL_DIR/"
else
    cp "$PROJECT_DIR/target/release/tmux-bw-ssh" "$INSTALL_DIR/"
fi
chmod +x "$INSTALL_DIR/tmux-bw-ssh"

# Asigurare PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
    export PATH="$INSTALL_DIR:$PATH"
fi

# 4. Configurare RBW (Prompt)
echo -e "
${BLUE}🔐 Configurare Vaultwarden / Bitwarden${NC}"
read -p "Introdu URL-ul serverului (ex: https://vault.exemplu.ro): " VAULT_URL
if [ -z "$VAULT_URL" ]; then
    echo "Skipping URL config..."
else
    rbw config set base_url "$VAULT_URL"
fi

read -p "Introdu adresa de email: " RBW_EMAIL
if [ -z "$RBW_EMAIL" ]; then
    echo "Skipping email config..."
else
    rbw config set email "$RBW_EMAIL"
fi

echo -e "
${GREEN}Instalare finalizata cu succes!${NC}"
echo -e "Pasi ramasi:"
echo -e "1. ${YELLOW}rbw login${NC} (pentru autentificare)"
echo -e "2. ${YELLOW}rbw unlock${NC} (pentru a porni sesiunea)"
echo -e "3. Ruleaza ${YELLOW}$PROJECT_DIR/tmux-bw-ssh.tmux${NC} in interiorul Tmux pentru a activa tastele."
