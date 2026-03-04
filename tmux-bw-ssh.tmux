#!/usr/bin/env bash

# Obținem calea absolută a directorului unde se află acest script
CURRENT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Setăm binarul și scriptul de inserare
BINARY="$CURRENT_DIR/target/release/tmux-bw-ssh"
INSERT_SCRIPT="$CURRENT_DIR/scripts/insert.sh"

# Aplicăm scurtăturile de taste în Tmux
# Prefix + S (Secure SSH) - Deschide selecția
tmux bind-key "S" new-window -n "tssh" "$BINARY"

# Prefix + p (password) - Inserare parolă
tmux bind-key "p" run-shell "$INSERT_SCRIPT"

# Notificare vizuală în status bar-ul tmux
tmux display-message "Tmux-BW-SSH bindings active: Prefix+S (SSH), Prefix+p (Pass)"
