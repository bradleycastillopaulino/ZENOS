//! Claves API en el Administrador de credenciales de Windows (vía `keyring`).
//! Las claves nunca se devuelven al frontend; solo se informa si existen.

const SERVICE: &str = "AI Star";

fn entry(provider: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, provider).map_err(|e| e.to_string())
}

pub fn get(provider: &str) -> Option<String> {
    entry(provider).ok()?.get_password().ok().filter(|k| !k.trim().is_empty())
}

pub fn set(provider: &str, key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("La clave está vacía.".into());
    }
    entry(provider)?.set_password(key).map_err(|e| e.to_string())
}

pub fn delete(provider: &str) -> Result<(), String> {
    match entry(provider)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

pub fn has(provider: &str) -> bool {
    get(provider).is_some()
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn windows_credential_manager_roundtrip() {
        let id = "aistar-ci-test";
        super::set(id, "  clave-de-prueba  ").unwrap();
        assert_eq!(super::get(id).as_deref(), Some("clave-de-prueba"));
        super::delete(id).unwrap();
        assert!(!super::has(id));
        super::delete(id).unwrap(); // borrar dos veces no falla
    }
}
