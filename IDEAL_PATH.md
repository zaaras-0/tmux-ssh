# IDEAL PATH - zbw (tmux-bw-ssh)

Acest document reflectă starea curentă a proiectului și deciziile arhitecturale luate pentru a asigura stabilitatea și compatibilitatea Bitwarden/Vaultwarden.

## 🏗️ Arhitectură Core
- **Limbaj:** Rust (Async cu `tokio`).
- **SDK:** Utilizare directă a `bitwarden-sdk` (crate-urile `core`, `api-api`, `crypto`, `encoding`).
- **Zero Binary Dependency:** Nu mai depindem de `bw` sau `rbw`. Totul este compilat în binarul `zbw`.

## 🔐 Securitate și Criptografie
- **Sesiune:** Salvată în `/dev/shm/zbw.session.json` (doar în RAM).
- **Manual Auth Flow:** Implementat manual `Prelogin` -> `MasterKey` -> `PasswordHash` -> `Identity Token` pentru compatibilitate cu Vaultwarden (evitând endpoint-urile `/password` noi din SDK).
- **Decriptare Hibridă:**
    - **User Key:** Decriptată cu `MasterKey`.
    - **Private Key:** Decriptată cu `UserKey`.
    - **Org Keys:** Decriptate asimetric (RSA) folosind `PrivateKey` sau simetric folosind `UserKey`.
- **Data Source:** Se folosește exclusiv endpoint-ul `/api/sync` pentru a prelua iteme, foldere, organizații și colecții într-un singur apel stabil.

## ⌨️ Integrare Tmux
- **Servers (S/s):** `new-window` pentru un terminal SSH dedicat.
- **Snippets (G/g):** `split-window` jos (30%). Injectează în `last_pane_id` și face auto-kill la pane-ul de selecție.
- **Search (/):** `split-window` cu prompt de căutare interactiv în bara de status.
- **Password Completion:** Parola este salvată în variabila de pane `@server_pass`. Poate fi re-injectată cu `Ctrl + P` (fără prefix) sau `Prefix + p`.

## 🚀 Direcții Viitoare
- [ ] Implementare suport 2FA în fluxul manual.
- [ ] Sistem de caching criptat pentru datele de sync (pornire instantanee offline-first).
- [ ] Selectare IP multiplu pentru un singur item Bitwarden.
- [ ] Comandă `zbw add` pentru a adăuga servere noi direct din CLI.

## 🛠️ Mentenanță
- **Build:** `cargo build --release`.
- **Install:** `bash tmux-ssh-install.sh`.
- **Reset Config:** `zbw purge`.
- **Lock Vault:** `zbw lock`.
