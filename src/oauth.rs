use crate::error::SlkError;
use std::process::Command;

const REDIRECT_URI: &str = "https://localhost:9876";

fn extract_code_from_url(url: &str) -> Result<String, SlkError> {
    let query = url
        .split('?')
        .nth(1)
        .ok_or(SlkError::from("no query string in callback URL"))?;

    for param in query.split('&') {
        if let Some(value) = param.strip_prefix("code=") {
            if !value.is_empty() {
                return Ok(value.to_string());
            }
        }
    }

    Err(SlkError::from(
        "no 'code' parameter in callback. Authorization may have been denied.",
    ))
}

fn exchange_code(
    client_id: &str,
    client_secret: &str,
    code: &str,
) -> Result<String, SlkError> {
    let output = Command::new("curl")
        .args([
            "-s",
            "-X",
            "POST",
            "-d",
            &format!(
                "client_id={}&client_secret={}&code={}&redirect_uri={}",
                client_id, client_secret, code, REDIRECT_URI
            ),
            "https://slack.com/api/oauth.v2.access",
        ])
        .output()
        .map_err(|e| SlkError::from(format!("failed to execute curl: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SlkError::from(format!(
            "curl failed (exit {}): {}",
            output.status, stderr
        )));
    }

    let body = String::from_utf8(output.stdout)
        .map_err(|e| SlkError::from(format!("invalid UTF-8 in response: {}", e)))?;

    let json_val = crate::json::parse(&body)?;

    let ok = json_val
        .get("ok")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !ok {
        let error = json_val
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(SlkError::from(format!(
            "oauth.v2.access failed: {}",
            error
        )));
    }

    let token = json_val
        .get("authed_user")
        .and_then(|u| u.get("access_token"))
        .and_then(|v| v.as_str())
        .ok_or(SlkError::from(
            "missing authed_user.access_token in response",
        ))?;

    Ok(token.to_string())
}

pub fn run_oauth_flow(client_id: &str, client_secret: &str) -> Result<String, SlkError> {
    let auth_url = format!(
        "https://slack.com/oauth/v2/authorize?client_id={}&user_scope=channels:history,groups:history,users:read&redirect_uri={}",
        client_id,
        REDIRECT_URI.replace(':', "%3A").replace('/', "%2F")
    );

    eprintln!("Open this URL in your browser:\n  {}", auth_url);
    let _ = Command::new("xdg-open").arg(&auth_url).spawn();

    eprintln!("\nAfter authorization, your browser will show a connection error.");
    eprintln!("Copy the URL from the address bar and paste it here:");

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| SlkError::from(format!("failed to read input: {}", e)))?;
    let input = input.trim();
    if input.is_empty() {
        return Err(SlkError::from("no URL provided"));
    }

    let code = extract_code_from_url(input)?;
    exchange_code(client_id, client_secret, &code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_code_valid() {
        let url = "https://localhost:9876?code=abc123def";
        assert_eq!(extract_code_from_url(url).unwrap(), "abc123def");
    }

    #[test]
    fn test_extract_code_multiple_params() {
        let url = "https://localhost:9876?state=xyz&code=mycode456";
        assert_eq!(extract_code_from_url(url).unwrap(), "mycode456");
    }

    #[test]
    fn test_extract_code_missing() {
        let url = "https://localhost:9876?error=access_denied";
        assert!(extract_code_from_url(url).is_err());
    }

    #[test]
    fn test_extract_code_no_query() {
        let url = "https://localhost:9876";
        assert!(extract_code_from_url(url).is_err());
    }
}
