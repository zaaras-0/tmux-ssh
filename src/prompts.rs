use anyhow::{Result, anyhow};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select, Password};
use console::Term;

/// Cere un string de la utilizator.
pub fn ask_input(label: &str, default: Option<String>) -> Result<String> {
    let theme = ColorfulTheme::default();
    
    // Inițializăm bbuilder-ul
    let mut input = Input::with_theme(&theme);
    
    // RE-ATRIBUIM rezultatul fiecărei metode înapoi variabilei 'input'
    input = input.with_prompt(label);
    
    if let Some(d) = default {
        input = input.default(d);
    }

    // Acum 'input' deține ultima versiune a obiectului
    input.interact_text().map_err(|e| anyhow!("Eroare la input: {}", e))
}

/// Întrebare de tip Yes/No.
pub fn ask_confirm(label: &str) -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(label)
        .default(false)
        .interact()
        .map_err(|e| anyhow!("Eroare la confirmare: {}", e))
}

/// Listă de selecție simplă (obligatorie).
pub fn select_from_list(label: &str, items: Vec<&str>) -> Result<String> {
    if items.is_empty() {
        return Err(anyhow!("Lista de selecție este goală pentru: {}", label));
    }

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(label)
        .items(&items)
        .default(0)
        .interact_on_opt(&Term::stderr())?;

    match selection {
        Some(index) => Ok(items[index].to_string()),
        None => Err(anyhow!("Selecție anulată (Ctrl+C)")),
    }
}

/// Listă de selecție cu un element implicit presetat.
pub fn select_from_list_with_default(label: &str, items: Vec<&str>, default_value: &str) -> Result<String> {
    if items.is_empty() {
        return Err(anyhow!("Lista de selecție este goală pentru: {}", label));
    }

    let default_index = items.iter()
        .position(|&x| x == default_value)
        .unwrap_or(0);

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(label)
        .items(&items)
        .default(default_index)
        .interact_on_opt(&Term::stderr())?;

    match selection {
        Some(index) => Ok(items[index].to_string()),
        None => Err(anyhow!("Selecție anulată (Ctrl+C)")),
    }
}

/// Prompt pentru parole.
pub fn ask_password(label: &str) -> Result<String> {
    Password::with_theme(&ColorfulTheme::default())
        .with_prompt(label)
        .interact()
        .map_err(|e| anyhow!("Eroare la citirea parolei: {}", e))
}