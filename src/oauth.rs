use crate::error::SlkError;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::sync::Arc;

use rustls::pki_types::PrivateKeyDer;
use rustls::ServerConfig;

const REDIRECT_URI: &str = "https://127.0.0.1:9876";

fn generate_state() -> Result<String, SlkError> {
    let mut buf = [0u8; 16];
    let mut f = std::fs::File::open("/dev/urandom")
        .map_err(|e| SlkError::from(format!("failed to open /dev/urandom: {}", e)))?;
    f.read_exact(&mut buf)
        .map_err(|e| SlkError::from(format!("failed to read /dev/urandom: {}", e)))?;
    Ok(buf.iter().map(|b| format!("{:02x}", b)).collect())
}

fn extract_callback_params(request: &str) -> Result<(String, String), SlkError> {
    let path = request
        .split_whitespace()
        .nth(1)
        .ok_or(SlkError::from("invalid HTTP request"))?;

    let query = path
        .split('?')
        .nth(1)
        .ok_or(SlkError::from("no query string in callback"))?;

    let mut code = None;
    let mut state = None;

    for param in query.split('&') {
        if let Some(value) = param.strip_prefix("code=") {
            if !value.is_empty() {
                code = Some(value.to_string());
            }
        } else if let Some(value) = param.strip_prefix("state=")
            && !value.is_empty()
        {
            state = Some(value.to_string());
        }
    }

    let code = code.ok_or(SlkError::from(
        "no 'code' parameter in callback. Authorization may have been denied.",
    ))?;
    let state = state.ok_or(SlkError::from("no 'state' parameter in callback"))?;

    Ok((code, state))
}

fn build_tls_config() -> Result<ServerConfig, SlkError> {
    let key_pair = rcgen::KeyPair::generate()
        .map_err(|e| SlkError::from(format!("failed to generate key pair: {}", e)))?;
    let cert = rcgen::CertificateParams::new(vec!["127.0.0.1".to_string()])
        .map_err(|e| SlkError::from(format!("failed to create cert params: {}", e)))?
        .self_signed(&key_pair)
        .map_err(|e| SlkError::from(format!("failed to generate self-signed cert: {}", e)))?;

    let cert_der = cert.der().clone();
    let key_der = PrivateKeyDer::Pkcs8(key_pair.serialize_der().into());

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)
        .map_err(|e| SlkError::from(format!("failed to build TLS config: {}", e)))?;
    Ok(config)
}

fn wait_for_callback(tls_config: Arc<ServerConfig>) -> Result<String, SlkError> {
    let listener = TcpListener::bind("127.0.0.1:9876")
        .map_err(|e| SlkError::from(format!("failed to bind port 9876: {}", e)))?;
    eprintln!("Waiting for callback on https://127.0.0.1:9876 ...");

    loop {
        let (tcp_stream, _) = listener
            .accept()
            .map_err(|e| SlkError::from(format!("failed to accept connection: {}", e)))?;
        let tls_conn = rustls::ServerConnection::new(Arc::clone(&tls_config))
            .map_err(|e| SlkError::from(format!("failed to create TLS connection: {}", e)))?;
        let mut stream = rustls::StreamOwned::new(tls_conn, tcp_stream);

        let mut buf = [0u8; 2048];
        let n = match stream.read(&mut buf) {
            Ok(n) if n > 0 => n,
            _ => continue,
        };
        let request = String::from_utf8_lossy(&buf[..n]).to_string();

        let response_body = "<html><body><h1>Authorization successful!</h1><p>You can close this tab.</p></body></html>";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(), response_body
        );
        let _ = stream.write_all(response.as_bytes());
        stream.conn.send_close_notify();
        let _ = stream.conn.write_tls(&mut stream.sock);

        return Ok(request);
    }
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
    let state = generate_state()?;
    let tls_config = Arc::new(build_tls_config()?);

    let auth_url = format!(
        "https://slack.com/oauth/v2/authorize?client_id={}&user_scope=channels:history,channels:read,groups:history,groups:read,mpim:read,im:read,users:read&redirect_uri={}&state={}",
        client_id,
        REDIRECT_URI.replace(':', "%3A").replace('/', "%2F"),
        state
    );

    eprintln!("Opening browser for authorization...");
    eprintln!("If prompted about the certificate, click 'Advanced' and 'Proceed'.");
    eprintln!("If the browser doesn't open, visit:\n  {}", auth_url);
    let _ = Command::new("xdg-open").arg(&auth_url).spawn();

    let request = wait_for_callback(tls_config)?;
    let (code, callback_state) = extract_callback_params(&request)?;

    if callback_state != state {
        return Err(SlkError::from(
            "state mismatch: possible CSRF attack. Please try again.",
        ));
    }

    exchange_code(client_id, client_secret, &code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_callback_params_valid() {
        let request = "GET /?code=abc123&state=deadbeef HTTP/1.1\r\nHost: localhost\r\n";
        let (code, state) = extract_callback_params(request).unwrap();
        assert_eq!(code, "abc123");
        assert_eq!(state, "deadbeef");
    }

    #[test]
    fn test_extract_callback_params_reversed_order() {
        let request = "GET /?state=mystate&code=mycode HTTP/1.1\r\n";
        let (code, state) = extract_callback_params(request).unwrap();
        assert_eq!(code, "mycode");
        assert_eq!(state, "mystate");
    }

    #[test]
    fn test_extract_callback_params_missing_code() {
        let request = "GET /?state=abc HTTP/1.1\r\n";
        let err = extract_callback_params(request).unwrap_err();
        assert!(err.message.contains("code"));
    }

    #[test]
    fn test_extract_callback_params_missing_state() {
        let request = "GET /?code=abc HTTP/1.1\r\n";
        let err = extract_callback_params(request).unwrap_err();
        assert!(err.message.contains("state"));
    }

    #[test]
    fn test_extract_callback_params_no_query() {
        let request = "GET / HTTP/1.1\r\n";
        assert!(extract_callback_params(request).is_err());
    }

    #[test]
    fn test_extract_callback_params_empty_request() {
        assert!(extract_callback_params("").is_err());
    }

    #[test]
    fn test_generate_state_length_and_hex() {
        let state = generate_state().unwrap();
        assert_eq!(state.len(), 32);
        assert!(state.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_state_unique() {
        let s1 = generate_state().unwrap();
        let s2 = generate_state().unwrap();
        assert_ne!(s1, s2);
    }
}
