use keyring::Entry;

const SERVICE: &str = "com.justindu.jobtracker";
const USERNAME: &str = "anthropic-api-key";

fn entry() -> Result<Entry, String> {
    Entry::new(SERVICE, USERNAME).map_err(|e| format!("Keychain unavailable: {e}"))
}

/// True if an API key is currently stored in the OS keychain.
pub fn has_api_key() -> bool {
    match entry() {
        Ok(e) => e.get_password().is_ok(),
        Err(_) => false,
    }
}

pub fn get_api_key() -> Result<String, String> {
    let e = entry()?;
    e.get_password()
        .map_err(|_| "No API key is stored yet.".to_string())
}

pub fn set_api_key(key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("API key cannot be empty.".to_string());
    }
    let e = entry()?;
    e.set_password(key)
        .map_err(|err| format!("Failed to store API key in the OS keychain: {err}"))
}

pub fn delete_api_key() -> Result<(), String> {
    let e = entry()?;
    match e.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(format!("Failed to remove API key: {err}")),
    }
}
