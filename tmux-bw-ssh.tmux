#!/usr/bin/env bash

# Obținem calea absolută a directorului unde se află acest script
CURRENT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Setăm binarul și scriptul de inserare
BINARY="$CURRENT_DIR/target/release/tmux-bw-ssh"
INSERT_SCRIPT="$CURRENT_DIR/scripts/insert.sh"

# Aplicăm scurtăturile de taste în Tmux

# Prefix + S (Secure SSH) - Deschide selecția serverelor
tmux bind-key "S" new-window -n "tssh" "$BINARY"

# Prefix + G (Gist/Snippets) - Deschide selecția scripturilor
tmux bind-key "G" new-window -n "tsnp" "$BINARY --snippets"

# Ctrl + P (fără prefix) - Inserare parolă
tmux bind-key -n "C-p" run-shell "$INSERT_SCRIPT"

# Păstrăm și Prefix + p ca alternativă tradițională
tmux bind-key "p" run-shell "$INSERT_SCRIPT"

# Notificare vizuală în status bar-ul tmux
tmux display-message "Tmux-BW-SSH: Pfx+S (SSH), Pfx+G (Snippets), Ctrl+P (Pass)"
