# ZBW - Bitwarden CLI Power Tool (Rust)

## 🎯 Obiectiv
Un utilitar CLI unificat pentru gestionarea accesului SSH și a snippet-urilor folosind Bitwarden SDK nativ, optimizat pentru workflow-uri în Tmux pe sisteme Linux.

## 🏗️ Arhitectură
- **Limbaj:** Rust (Async cu Tokio)
- **Backend:** Bitwarden SDK nativ (Rust).
- **Interfață:** `skim` (fuzzy finder) integrat în `tmux display-popup` pentru o experiență modală fluidă.
- **Sesiune:** Stocată exclusiv în RAM (`/dev/shm/zbw.session.json`) pentru securitate.
- **Configurație:** JSON pe disk (`~/.config/zbw/config.json`).

## 🛠️ Module & Responsabilități
1. **`auth.rs`**: Gestionare sesiune (Login, Unlock, RAM session persistence via /dev/shm).
2. **`config.rs`**: Wizard interactiv pentru setup (Email, Server, Foldere Personalizate).
3. **`vault.rs`**: Extracție date prin `/api/sync` și filtrare hibridă (Personal + Orgs).
4. **`ssh.rs`**: Conectare SSH, redenumire fereastră Tmux, injectare parolă în pane.
5. **`snippets.rs`**: Injectare literală de text în pane-uri Tmux via popup.
6. **`models.rs`**: Structuri de date (Config, BwCipher, etc).
7. **`prompts.rs`**: Interacțiuni interactive (Select, Input, Confirm).

## 🔄 Fluxul de Configurare (Wizard)
1. **Identitate:** Email & Server URL (cu defaults din config existent).
2. **Sync:** Login temporar pentru interogarea structurii de foldere.
3. **Personal (Obligatoriu):** 
   - Selecție folder pentru SERVERE (Default: "Servers").
   - Selecție folder pentru SNIPPETS (Default: "Snippets").
4. **Organizations (Opțional):** Selecție organizații și colecții.
5. **Finalizare:** Salvare configurație.

## 📋 Comenzi Principale
- `zbw login / lock / unlock / purge / status`
- `zbw list / ssh` (Selector de servere)
- `zbw snippets / snip` (Selector de snippets)
- `zbw search <query>` (Căutare globală)
- `zbw config` (Wizard de configurare)
