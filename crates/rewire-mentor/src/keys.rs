//! API keys live only in the OS keychain (service `dev.playrewire.rewire`,
//! account = preset id). A small trait lets tests use memory instead.

use crate::MentorError;
use std::{collections::HashMap, sync::Mutex};

pub const SERVICE: &str = "dev.playrewire.rewire";

pub trait KeyStore {
    fn set(&self, account: &str, key: &str) -> Result<(), MentorError>;
    /// `Ok(None)` means "no entry".
    fn get(&self, account: &str) -> Result<Option<String>, MentorError>;
}

/// The real OS keychain.
pub struct KeyringStore;

fn keyring_err(e: keyring::Error) -> MentorError {
    // keyring errors never include the secret; keep only the variant text.
    MentorError::Keychain(e.to_string())
}

impl KeyStore for KeyringStore {
    fn set(&self, account: &str, key: &str) -> Result<(), MentorError> {
        keyring::Entry::new(SERVICE, account).and_then(|e| e.set_password(key)).map_err(keyring_err)
    }
    fn get(&self, account: &str) -> Result<Option<String>, MentorError> {
        match keyring::Entry::new(SERVICE, account).and_then(|e| e.get_password()) {
            Ok(k) => Ok(Some(k)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(keyring_err(e)),
        }
    }
}

/// In-memory store for tests. Never touches the user's keychain.
#[derive(Default)]
pub struct MemoryStore(Mutex<HashMap<String, String>>);

impl KeyStore for MemoryStore {
    fn set(&self, account: &str, key: &str) -> Result<(), MentorError> {
        self.0.lock().unwrap().insert(account.into(), key.into());
        Ok(())
    }
    fn get(&self, account: &str) -> Result<Option<String>, MentorError> {
        Ok(self.0.lock().unwrap().get(account).cloned())
    }
}

pub fn store_key(preset_id: &str, key: &str) -> Result<(), MentorError> {
    KeyringStore.set(preset_id, key)
}

pub fn load_key(preset_id: &str) -> Result<Option<String>, MentorError> {
    KeyringStore.get(preset_id)
}
