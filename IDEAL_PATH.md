# Parcursul Ideal: tmux-bw-ssh

Acest document definește arhitectura și fluxul logic al instrumentului, bazat pe principiile securității prin izolare și vitezei de execuție.

## Principiile de Bază
1. **Securitate prin Izolare:** Secretele nu ating niciodată clipboard-ul global. Sunt stocate în memoria procesului `tmux` (variabile de panou).
2. **Viteză (rbw):** Utilizarea `rbw` (Rust Bitwarden) pentru acces instantaneu prin agentul de autentificare.
3. **Interfață Fuzzy:** Selecție rapidă a serverelor folosind `skim`.

## Fluxul Logic (Arhitectura)

### 1. Sursa de Adevăr (Bitwarden / rbw)
- Folosește `rbw` pentru viteză nativă.
- Presupune existența unui folder numit `Servers` în Bitwarden.
- Agentul `rbw` păstrează master password-ul în RAM (criptat).

### 2. Selecția (Skim)
- Binarul Rust injectează lista de servere din folderul `Servers` în `skim`.
- UI-ul este limitat la 40% din înălțimea terminalului, poziționat inversat pentru vizibilitate.

### 3. Conexiune și Metadate
- La selecție, binarul:
    - Setează variabila `@server_pass` în panoul curent (pane-local).
    - Redenumește fereastra `tmux` după numele serverului.
    - Execută `ssh` înlocuind procesul curent (`exec`).

### 4. Autentificare "La Cerere"
- **Keybind:** `Prefix + p`
- Execută un script (`insert.sh`) care citește `@server_pass` și o trimite prin `send-keys` direct în fluxul standard al panoului.

### 5. Securitate Post-Sesiune
- Metadatele (`@server_pass`) sunt legate de ID-ul panoului. La închiderea panoului/sesiunii, secretul este distrus automat de către `tmux`.

---
*Acest document servește drept specificație tehnică obligatorie pentru dezvoltarea `tmux-bw-ssh`.*
