use crate::error::SlkError;
use std::fs;
use std::path::PathBuf;

pub fn config_dir() -> Result<PathBuf, SlkError> {
    let base = match std::env::var("XDG_CONFIG_HOME") {
        Ok(val) if !val.is_empty() => PathBuf::from(val),
        _ => {
            let home = std::env::var("HOME")
                .map_err(|_| SlkError::from("HOME environment variable is not set"))?;
            PathBuf::from(home).join(".config")
        }
    };
    Ok(base.join("slk"))
}

pub fn load_token() -> Result<Option<String>, SlkError> {
    let path = config_dir()?.join("credentials");
    match fs::read_to_string(&path) {
        Ok(contents) => {
            let token = contents.trim().to_string();
            if token.is_empty() {
                Ok(None)
            } else {
                Ok(Some(token))
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(SlkError::from(format!(
            "failed to read {}: {}",
            path.display(),
            e
        ))),
    }
}

pub fn save_token(token: &str) -> Result<PathBuf, SlkError> {
    let dir = config_dir()?;
    fs::create_dir_all(&dir).map_err(|e| {
        SlkError::from(format!(
            "failed to create directory {}: {}",
            dir.display(),
            e
        ))
    })?;

    let path = dir.join("credentials");
    fs::write(&path, token).map_err(|e| {
        SlkError::from(format!(
            "failed to write {}: {}",
            path.display(),
            e
        ))
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&path, perms).map_err(|e| {
            SlkError::from(format!(
                "failed to set permissions on {}: {}",
                path.display(),
                e
            ))
        })?;
    }

    Ok(path)
}

pub fn load_client_credentials() -> Result<(String, String), SlkError> {
    if let (Ok(id), Ok(secret)) = (
        std::env::var("SLK_CLIENT_ID"),
        std::env::var("SLK_CLIENT_SECRET"),
    ) {
        if !id.is_empty() && !secret.is_empty() {
            return Ok((id, secret));
        }
    }

    let path = config_dir()?.join("config.json");
    let contents = fs::read_to_string(&path).map_err(|_| {
        SlkError::from(
            "client_id and client_secret are required. Set SLK_CLIENT_ID/SLK_CLIENT_SECRET or create ~/.config/slk/config.json",
        )
    })?;

    let json_val = crate::json::parse(&contents)?;
    let client_id = json_val
        .get("client_id")
        .and_then(|v| v.as_str())
        .ok_or(SlkError::from("missing 'client_id' in config.json"))?
        .to_string();
    let client_secret = json_val
        .get("client_secret")
        .and_then(|v| v.as_str())
        .ok_or(SlkError::from("missing 'client_secret' in config.json"))?
        .to_string();

    Ok((client_id, client_secret))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_dir_uses_home() {
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
        let dir = config_dir().unwrap();
        assert!(dir.ends_with(".config/slk"));
    }

    #[test]
    fn test_config_dir_uses_xdg() {
        unsafe { std::env::set_var("XDG_CONFIG_HOME", "/tmp/test-xdg") };
        let dir = config_dir().unwrap();
        assert_eq!(dir, PathBuf::from("/tmp/test-xdg/slk"));
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
    }

    #[test]
    fn test_load_token_missing_file() {
        unsafe { std::env::set_var("XDG_CONFIG_HOME", "/tmp/slk-test-nonexistent") };
        let result = load_token().unwrap();
        assert_eq!(result, None);
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
    }

    #[test]
    fn test_save_and_load_token() {
        let tmp = std::env::temp_dir().join("slk-test-save-load");
        let _ = fs::remove_dir_all(&tmp);
        unsafe { std::env::set_var("XDG_CONFIG_HOME", &tmp) };

        let path = save_token("xoxp-test-token-123").unwrap();
        assert!(path.exists());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }

        let token = load_token().unwrap();
        assert_eq!(token, Some("xoxp-test-token-123".to_string()));

        let _ = fs::remove_dir_all(&tmp);
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
    }

    #[test]
    fn test_load_client_credentials_from_env() {
        unsafe { std::env::set_var("SLK_CLIENT_ID", "env-id") };
        unsafe { std::env::set_var("SLK_CLIENT_SECRET", "env-secret") };
        let (id, secret) = load_client_credentials().unwrap();
        assert_eq!(id, "env-id");
        assert_eq!(secret, "env-secret");
        unsafe { std::env::remove_var("SLK_CLIENT_ID") };
        unsafe { std::env::remove_var("SLK_CLIENT_SECRET") };
    }
}
