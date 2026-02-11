use crate::error::SlkError;
use crate::json::JsonValue;

#[derive(Debug, PartialEq)]
pub struct SlackMessage {
    pub user: String,
    pub text: String,
    pub ts: String,
}

pub fn format_unix_ts(ts_str: &str) -> String {
    let secs: i64 = match ts_str.split('.').next() {
        Some(s) => s.parse().unwrap_or(0),
        None => 0,
    };

    let time_of_day = secs.rem_euclid(86400);
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Howard Hinnant's civil_from_days algorithm
    let z = secs.div_euclid(86400) + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        y, m, d, hours, minutes, seconds
    )
}

pub fn extract_messages(response: &JsonValue) -> Result<Vec<SlackMessage>, SlkError> {
    let ok = response
        .get("ok")
        .and_then(|v| v.as_bool())
        .ok_or(SlkError::from("missing 'ok' field in response"))?;

    if !ok {
        let error = response
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        let needed = response.get("needed").and_then(|v| v.as_str());
        let provided = response.get("provided").and_then(|v| v.as_str());
        let mut msg = format!("Slack API error: {}", error);
        if let Some(needed) = needed {
            msg.push_str(&format!("\n  needed scope: {}", needed));
        }
        if let Some(provided) = provided {
            msg.push_str(&format!("\n  provided scopes: {}", provided));
        }
        return Err(SlkError::from(msg));
    }

    let messages = response
        .get("messages")
        .and_then(|v| v.as_array())
        .ok_or(SlkError::from("missing 'messages' array in response"))?;

    let mut result = Vec::new();
    for msg in messages {
        let user = msg
            .get("user")
            .and_then(|v| v.as_str())
            .or_else(|| msg.get("username").and_then(|v| v.as_str()))
            .or_else(|| msg.get("bot_id").and_then(|v| v.as_str()))
            .unwrap_or("unknown")
            .to_string();

        let text = msg
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let ts = msg
            .get("ts")
            .and_then(|v| v.as_str())
            .unwrap_or("0")
            .to_string();

        result.push(SlackMessage { user, text, ts });
    }

    Ok(result)
}

#[derive(Debug, PartialEq)]
pub struct SlackConversation {
    pub id: String,
    pub name: String,
}

pub fn extract_conversations(response: &JsonValue) -> Result<Vec<SlackConversation>, SlkError> {
    let ok = response
        .get("ok")
        .and_then(|v| v.as_bool())
        .ok_or(SlkError::from("missing 'ok' field in response"))?;

    if !ok {
        let error = response
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        let needed = response.get("needed").and_then(|v| v.as_str());
        let provided = response.get("provided").and_then(|v| v.as_str());
        let mut msg = format!("Slack API error: {}", error);
        if let Some(needed) = needed {
            msg.push_str(&format!("\n  needed scope: {}", needed));
        }
        if let Some(provided) = provided {
            msg.push_str(&format!("\n  provided scopes: {}", provided));
        }
        return Err(SlkError::from(msg));
    }

    let channels = response
        .get("channels")
        .and_then(|v| v.as_array())
        .ok_or(SlkError::from("missing 'channels' array in response"))?;

    let mut result = Vec::new();
    for ch in channels {
        let id = ch
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let name = ch
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        result.push(SlackConversation { id, name });
    }

    Ok(result)
}

pub fn resolve_user_name(response: &JsonValue) -> Result<String, SlkError> {
    let ok = response
        .get("ok")
        .and_then(|v| v.as_bool())
        .ok_or(SlkError::from("missing 'ok' field in response"))?;

    if !ok {
        let error = response
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        let needed = response.get("needed").and_then(|v| v.as_str());
        let provided = response.get("provided").and_then(|v| v.as_str());
        let mut msg = format!("Slack API error: {}", error);
        if let Some(needed) = needed {
            msg.push_str(&format!("\n  needed scope: {}", needed));
        }
        if let Some(provided) = provided {
            msg.push_str(&format!("\n  provided scopes: {}", provided));
        }
        return Err(SlkError::from(msg));
    }

    let user = response
        .get("user")
        .ok_or(SlkError::from("missing 'user' field in response"))?;

    let profile = user.get("profile");
    if let Some(profile) = profile {
        if let Some(display_name) = profile.get("display_name").and_then(|v| v.as_str()) {
            if !display_name.is_empty() {
                return Ok(display_name.to_string());
            }
        }
    }

    if let Some(real_name) = user.get("real_name").and_then(|v| v.as_str()) {
        if !real_name.is_empty() {
            return Ok(real_name.to_string());
        }
    }

    if let Some(name) = user.get("name").and_then(|v| v.as_str()) {
        if !name.is_empty() {
            return Ok(name.to_string());
        }
    }

    Err(SlkError::from("no user name found in response"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json;

    #[test]
    fn test_extract_messages() {
        let input = r#"{
            "ok": true,
            "messages": [
                {"user": "U081R4ZS5E2", "text": "Hello, this is a thread", "ts": "1770689887.565249"},
                {"user": "U092X3AB7F1", "text": "Great thread!", "ts": "1770689900.000100"}
            ],
            "has_more": false
        }"#;
        let json_val = json::parse(input).unwrap();
        let messages = extract_messages(&json_val).unwrap();

        assert_eq!(messages.len(), 2);
        assert_eq!(
            messages[0],
            SlackMessage {
                user: "U081R4ZS5E2".to_string(),
                text: "Hello, this is a thread".to_string(),
                ts: "1770689887.565249".to_string(),
            }
        );
        assert_eq!(
            messages[1],
            SlackMessage {
                user: "U092X3AB7F1".to_string(),
                text: "Great thread!".to_string(),
                ts: "1770689900.000100".to_string(),
            }
        );
    }

    #[test]
    fn test_api_error_response() {
        let input = r#"{"ok": false, "error": "channel_not_found"}"#;
        let json_val = json::parse(input).unwrap();
        let result = extract_messages(&json_val);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("channel_not_found"));
    }

    #[test]
    fn test_missing_user_falls_back_to_username() {
        let input = r#"{
            "ok": true,
            "messages": [{"username": "bot_name", "text": "bot message"}]
        }"#;
        let json_val = json::parse(input).unwrap();
        let messages = extract_messages(&json_val).unwrap();

        assert_eq!(messages[0].user, "bot_name");
    }

    #[test]
    fn test_missing_user_falls_back_to_bot_id() {
        let input = r#"{
            "ok": true,
            "messages": [{"bot_id": "B123", "text": "bot message"}]
        }"#;
        let json_val = json::parse(input).unwrap();
        let messages = extract_messages(&json_val).unwrap();

        assert_eq!(messages[0].user, "B123");
    }

    #[test]
    fn test_missing_text_uses_empty_string() {
        let input = r#"{
            "ok": true,
            "messages": [{"user": "U123"}]
        }"#;
        let json_val = json::parse(input).unwrap();
        let messages = extract_messages(&json_val).unwrap();

        assert_eq!(messages[0].text, "");
    }

    #[test]
    fn test_resolve_user_name_display_name() {
        let input = r#"{
            "ok": true,
            "user": {
                "name": "kanta",
                "real_name": "Kanta Otomaeru",
                "profile": {
                    "display_name": "kanta"
                }
            }
        }"#;
        let json_val = json::parse(input).unwrap();
        assert_eq!(resolve_user_name(&json_val).unwrap(), "kanta");
    }

    #[test]
    fn test_resolve_user_name_fallback_to_real_name() {
        let input = r#"{
            "ok": true,
            "user": {
                "name": "kanta",
                "real_name": "Kanta Otomaeru",
                "profile": {
                    "display_name": ""
                }
            }
        }"#;
        let json_val = json::parse(input).unwrap();
        assert_eq!(resolve_user_name(&json_val).unwrap(), "Kanta Otomaeru");
    }

    #[test]
    fn test_resolve_user_name_fallback_to_name() {
        let input = r#"{
            "ok": true,
            "user": {
                "name": "kanta",
                "profile": {
                    "display_name": ""
                }
            }
        }"#;
        let json_val = json::parse(input).unwrap();
        assert_eq!(resolve_user_name(&json_val).unwrap(), "kanta");
    }

    #[test]
    fn test_resolve_user_name_api_error() {
        let input = r#"{"ok": false, "error": "user_not_found"}"#;
        let json_val = json::parse(input).unwrap();
        let result = resolve_user_name(&json_val);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("user_not_found"));
    }

    #[test]
    fn test_resolve_user_name_missing_scope() {
        let input = r#"{
            "ok": false,
            "error": "missing_scope",
            "needed": "users:read",
            "provided": "channels:history"
        }"#;
        let json_val = json::parse(input).unwrap();
        let result = resolve_user_name(&json_val);
        assert!(result.is_err());
        let msg = result.unwrap_err().message;
        assert!(msg.contains("missing_scope"));
        assert!(msg.contains("users:read"));
        assert!(msg.contains("channels:history"));
    }

    #[test]
    fn test_completely_unknown_user() {
        let input = r#"{
            "ok": true,
            "messages": [{"text": "orphan message"}]
        }"#;
        let json_val = json::parse(input).unwrap();
        let messages = extract_messages(&json_val).unwrap();

        assert_eq!(messages[0].user, "unknown");
    }

    #[test]
    fn test_extract_conversations() {
        let input = r#"{
            "ok": true,
            "channels": [
                {"id": "C081VT5GLQH", "name": "general"},
                {"id": "C092X3AB7F1", "name": "random"}
            ]
        }"#;
        let json_val = json::parse(input).unwrap();
        let conversations = extract_conversations(&json_val).unwrap();

        assert_eq!(conversations.len(), 2);
        assert_eq!(
            conversations[0],
            SlackConversation {
                id: "C081VT5GLQH".to_string(),
                name: "general".to_string(),
            }
        );
        assert_eq!(
            conversations[1],
            SlackConversation {
                id: "C092X3AB7F1".to_string(),
                name: "random".to_string(),
            }
        );
    }

    #[test]
    fn test_extract_conversations_error() {
        let input = r#"{"ok": false, "error": "invalid_auth"}"#;
        let json_val = json::parse(input).unwrap();
        let result = extract_conversations(&json_val);

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("invalid_auth"));
    }

    #[test]
    fn test_extract_conversations_empty() {
        let input = r#"{"ok": true, "channels": []}"#;
        let json_val = json::parse(input).unwrap();
        let conversations = extract_conversations(&json_val).unwrap();

        assert!(conversations.is_empty());
    }
}
