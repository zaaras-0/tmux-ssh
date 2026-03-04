#!/usr/bin/env bash

# Obținem parola din variabila locală a panoului (pane-local option)
# Folosim -p pentru pane-local, -v pentru valoare pură
PASS=$(tmux show-options -pv @server_pass)

if [ -n "$PASS" ]; then
    # Trimitem parola urmată de Enter direct în panoul curent
    tmux send-keys "$PASS" Enter
    tmux display-message "Password injected securely 🔐"
else
    # Mesaj de eroare dacă nu există nicio parolă stocată
    tmux display-message "❌ No password found for this pane (@server_pass)"
fi
