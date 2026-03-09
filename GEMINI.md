# Mandate Proiect: tmux-bw-ssh

Toate modificările aduse acestui codebase trebuie să respecte specificațiile din `IDEAL_PATH.md`.

## Reguli de Aur
- **Fără Clipboard:** Nu folosi niciodată `xclip`, `pbcopy` sau clipboard-ul tmux pentru parole.
- **Performanță Rust:** Păstrează dependențele la minimum. Preferă executabilele native (rbw) în locul bibliotecilor grele de API.
- **UX Tmux:** Păstrează scurtăturile de taste consistente cu fluxul de lucru SSH (ex: `Prefix + p` pentru parolă).
- **Compilare:** Asigură-te că binarul este compilat în modul `--release` pentru viteză.

## Context Execuție
- Binarul: `target/release/tmux-bw-ssh`
- Script Inserare: `scripts/insert.sh`
- Script Instalare: `tmux-ssh-install.sh`
