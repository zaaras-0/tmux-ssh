# ⚡ ZBW - Bitwarden CLI Power Tool for Tmux

ZBW is a high-performance Rust utility designed for developers and sysadmins who live in Tmux. It integrates Bitwarden securely with your terminal workflow, providing instant access to SSH servers and snippets with automated password injection.

---

## 🚀 Quick Install

Run the following command in your terminal to download and install ZBW:

```bash
curl -sSL https://raw.githubusercontent.com/zaaras-0/tmux-ssh/main/tmux-ssh-install.sh | bash
```

*Note: This script will compile the binary from source (requires Rust/Cargo), add it to your PATH, and update your `~/.tmux.conf`.*

---

## ✨ Features

- **🔐 Secure Vault Access:** Native Bitwarden SDK integration with RAM-only session storage.
- **🖥️ SSH Selector:** Fuzzy search through your Bitwarden "Servers" folder and connect instantly.
- **📝 Snippets Manager:** Inject text snippets (notes) directly into your active Tmux pane.
- **🔑 Auto-Injection:** Automatically types your password after SSH connection (no more copy-pasting!).
- **🔍 Global Search:** Search your entire vault directly from a Tmux popup.
- **🛠️ Tmux Native:** Designed to work perfectly with `tmux display-popup` for a seamless modal experience.

---

## ⌨️ Default Keybindings (Tmux)

| Key | Action |
|-----|--------|
| `Prefix` + `s` | Open SSH Server Selector |
| `Prefix` + `g` | Open Snippets Selector |
| `Prefix` + `/` | Global Vault Search |
| `Ctrl` + `p` | Manual Password Injection (into current pane) |
| `Prefix` + `p` | Manual Password Injection (into current pane) |

---

## 🛠️ Configuration

After installation, run the setup wizard to connect your Bitwarden account:

```bash
zbw config
```

You will be asked for:
1. Bitwarden Email & Server URL.
2. Personal Folders for Servers and Snippets.
3. (Optional) Organization and Collection selections.

---

## ⚖️ License

This project is licensed under the **GNU GPL v3**.

- **Freedom:** You can use and modify this binary as you wish.
- **Reciprocity:** If you modify and redistribute it, you must publish the modified source code.
- **No Warranty:** The program is provided "as is" without any warranty.

See the [LICENSE](LICENSE) file for complete details.
