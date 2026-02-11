use crate::error::SlkError;
use std::process::Command;

pub fn build_api_url(channel_id: &str, ts: &str) -> String {
    format!(
        "https://slack.com/api/conversations.replies?channel={}&ts={}",
        channel_id, ts
    )
}

pub fn fetch_thread_replies(channel_id: &str, ts: &str, token: &str) -> Result<String, SlkError> {
    let url = build_api_url(channel_id, ts);
    let output = Command::new("curl")
        .args(["-s", "-H", &format!("Authorization: Bearer {}", token), &url])
        .output()
        .map_err(|e| SlkError::from(format!("failed to execute curl: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SlkError::from(format!(
            "curl failed (exit {}): {}",
            output.status, stderr
        )));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| SlkError::from(format!("invalid UTF-8 in response: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_api_url() {
        assert_eq!(
            build_api_url("C081VT5GLQH", "1770689887.565249"),
            "https://slack.com/api/conversations.replies?channel=C081VT5GLQH&ts=1770689887.565249"
        );
    }

    #[test]
    fn test_full_pipeline_with_recorded_response() {
        let recorded_json = r#"{
            "ok": true,
            "messages": [
                {
                    "user": "U081R4ZS5E2",
                    "type": "message",
                    "ts": "1770689887.565249",
                    "text": "Hello, this is a thread"
                },
                {
                    "user": "U092X3AB7F1",
                    "type": "message",
                    "ts": "1770689900.000100",
                    "text": "Great thread!"
                }
            ],
            "has_more": false
        }"#;

        let json_val = crate::json::parse(recorded_json).unwrap();
        let messages = crate::message::extract_messages(&json_val).unwrap();

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].user, "U081R4ZS5E2");
        assert_eq!(messages[0].text, "Hello, this is a thread");
        assert_eq!(messages[1].user, "U092X3AB7F1");
        assert_eq!(messages[1].text, "Great thread!");
    }

    #[test]
    fn test_pipeline_with_error_response() {
        let recorded_json = r#"{"ok": false, "error": "invalid_auth"}"#;

        let json_val = crate::json::parse(recorded_json).unwrap();
        let result = crate::message::extract_messages(&json_val);

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("invalid_auth"));
    }
}
