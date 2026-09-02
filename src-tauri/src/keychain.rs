use keyring::Entry;

const SERVICE: &str = "com.justindu.jobtracker";

/// Keychain account name for a provider's key.
///
/// Claude keeps the original `anthropic-api-key` account rather than moving
/// to a `claude-` name: an existing install already has a key stored there,
/// and renaming it would silently lose it.
fn account_for(provider: &str) -> String {
    match provider {
        "claude" => "anthropic-api-key".to_string(),
        other => format!("{other}-api-key"),
    }
}

fn entry(provider: &str) -> Result<Entry, String> {
    Entry::new(SERVICE, &account_for(provider))
        .map_err(|e| format!("Keychain unavailable: {e}"))
}

/// True if a key for this provider is currently in the OS keychain.
pub fn has_api_key(provider: &str) -> bool {
    match entry(provider) {
        Ok(e) => e.get_password().is_ok(),
        Err(_) => false,
    }
}

pub fn get_api_key(provider: &str) -> Result<String, String> {
    let e = entry(provider)?;
    e.get_password()
        .map_err(|_| format!("No API key is stored for {provider} yet."))
}

pub fn set_api_key(provider: &str, key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("API key cannot be empty.".to_string());
    }
    let e = entry(provider)?;
    e.set_password(key)
        .map_err(|err| format!("Failed to store API key in the OS keychain: {err}"))
}

pub fn delete_api_key(provider: &str) -> Result<(), String> {
    let e = entry(provider)?;
    match e.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(format!("Failed to remove API key: {err}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_keeps_the_account_name_the_app_shipped_with() {
        // Changing this would orphan the key of anyone who upgrades.
        assert_eq!(account_for("claude"), "anthropic-api-key");
    }

    #[test]
    fn each_provider_gets_its_own_account() {
        let accounts = ["claude", "openai", "gemini"].map(account_for);
        assert_eq!(accounts[1], "openai-api-key");
        assert_eq!(accounts[2], "gemini-api-key");
        let mut sorted = accounts.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 3, "provider keys must not share a slot");
    }
}
