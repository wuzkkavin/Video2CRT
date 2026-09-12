//! Windows Credential Manager integration via the `keyring` crate.
//!
//! We store the Minimax API key under service="Video2CRT" / user="default".
//! Never log, print, or return the key — the React UI only ever sees a
//! presence boolean (`has_api_key`).

use anyhow::Result;
use keyring::Entry;

const SERVICE: &str = "Video2CRT";
const USER: &str = "default";

fn entry() -> Result<Entry> {
    Ok(Entry::new(SERVICE, USER)?)
}

/// Save the API key. Overwrites any previous value.
pub fn save_api_key(key: &str) -> Result<()> {
    let e = entry()?;
    e.set_password(key)?;
    Ok(())
}

/// Return true iff a non-empty API key is stored.
pub fn has_api_key() -> bool {
    get_api_key().map(|k| !k.trim().is_empty()).unwrap_or(false)
}

/// Return the stored API key, or None when absent. Used internally only
/// (e.g. by the translator module when calling `/v1/models`).
pub fn get_api_key() -> Option<String> {
    match entry() {
        Ok(e) => e.get_password().ok().filter(|s| !s.is_empty()),
        Err(_) => None,
    }
}

/// Delete the stored API key. No-op if absent.
pub fn delete_api_key() -> Result<()> {
    let e = entry()?;
    // `delete_credential` swallows "not found" errors in some versions, but
    // we ignore errors here — the semantic is "ensure key is gone".
    let _ = e.delete_credential();
    Ok(())
}
