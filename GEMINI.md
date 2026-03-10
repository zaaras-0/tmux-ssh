# ZBW - Bitwarden CLI Power Tool (Rust)

## 🎯 Obiectiv
Un utilitar CLI unificat pentru gestionarea accesului SSH și a snippet-urilor folosind Bitwarden CLI oficial (`bw`), optimizat pentru workflow-uri în Tmux pe sisteme Linux.

## 🏗️ Arhitectură
- **Limbaj:** Rust
- **Backend:** Bitwarden CLI (`bw`) oficial via JSON.
- **Interfață:** `skim` (fuzzy finder) și `dialoguer` (wizard-uri interactive).
- **Sesiune:** Stocată exclusiv în RAM (`/dev/shm`) pentru securitate.
- **Configurație:** JSON pe disk (`~/.config/zbw/config.json`).

## 🛠️ Module & Responsabilități
1. **`auth.rs`**: Gestionare sesiune (Login, Unlock, RAM session persistence).
2. **`config.rs`**: Wizard interactiv pentru setup (Email, Server, Foldere/Colecții).
3. **`vault.rs`**: Extracție JSON și mapare ierarhică (Personal vs. Organizații).
4. **`ssh.rs`**: Conectare SSH, redenumire fereastră Tmux, thread nativ pentru injectare parolă.
5. **`snippets.rs`**: Injectare literală de text în panourile Tmux.
6. **`models.rs`**: Definiții structuri JSON și configurații.
7. **`prompts.rs`**: Interacțiuni interactive cu utilizatorul.

## 🔄 Fluxul de Configurare (Wizard)
1. **Identitate:** Email & Server URL.
2. **Sync:** Login temporar pentru interogarea structurii.
3. **Personal (Obligatoriu):** Selecție folder pentru servere/snippets.
4. **Organizations (Loop Opțional):** - Întreabă dacă se doresc setări pentru orgs.
   - Listare orgs -> Selecție org -> Selecție colecție.
   - Loop până la finalizare sau Ctrl+C.
5. **zbw Defaults:** Selecție folder pentru setări interne tool.

## 📋 Comenzi Planificate
- `zbw login / lock / unlock / purge`
- `zbw list / search`
- `zbw add / edit / get / rm`
- `zbw generate (p, P, u)`