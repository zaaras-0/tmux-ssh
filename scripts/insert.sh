#!/usr/bin/env bash
CURRENT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Compilează binarul la prima instalare
if [ ! -f "$CURRENT_DIR/target/release/tmux-bw-ssh" ]; then
    cd "$CURRENT_DIR" && cargo build --release >/dev/null 2>&1
fi

# Mapări: Prefix + s pentru selecție, Prefix + p pentru parolă
tmux bind-key "s" run-shell "$CURRENT_DIR/target/release/tmux-bw-ssh"
tmux bind-key "p" run-shell "tmux send-keys \"\$(tmux show-options -pv @server_pass)\""