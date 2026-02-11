use crate::error::SlkError;
use crate::json::JsonValue;

#[derive(Debug, PartialEq)]
pub struct SlackMessage {
    pub user: String,
    pub text: String,
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

        result.push(SlackMessage { user, text });
    }

    Ok(result)
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
            }
        );
        assert_eq!(
            messages[1],
            SlackMessage {
                user: "U092X3AB7F1".to_string(),
                text: "Great thread!".to_string(),
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
    fn test_completely_unknown_user() {
        let input = r#"{
            "ok": true,
            "messages": [{"text": "orphan message"}]
        }"#;
        let json_val = json::parse(input).unwrap();
        let messages = extract_messages(&json_val).unwrap();

        assert_eq!(messages[0].user, "unknown");
    }
}
