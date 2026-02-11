use crate::error::SlkError;

#[derive(Debug, PartialEq)]
pub struct SlackThread {
    pub channel_id: String,
    pub ts: String,
}

pub fn parse_slack_url(url: &str) -> Result<SlackThread, SlkError> {
    let segments: Vec<&str> = url.split('/').collect();

    let archives_pos = segments
        .iter()
        .position(|&s| s == "archives")
        .ok_or(SlkError::from("URL must contain '/archives/'"))?;

    let channel_id = segments
        .get(archives_pos + 1)
        .ok_or(SlkError::from("missing channel ID after /archives/"))?;

    let ts_segment = segments
        .get(archives_pos + 2)
        .ok_or(SlkError::from("missing timestamp after channel ID"))?;

    let ts = convert_timestamp(ts_segment)?;

    Ok(SlackThread {
        channel_id: channel_id.to_string(),
        ts,
    })
}

fn convert_timestamp(raw: &str) -> Result<String, SlkError> {
    let digits = raw
        .strip_prefix('p')
        .ok_or(SlkError::from("timestamp must start with 'p'"))?;

    if digits.len() <= 10 {
        return Err(SlkError::from("timestamp too short"));
    }

    let (seconds, micros) = digits.split_at(10);
    Ok(format!("{}.{}", seconds, micros))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_url() {
        let result = parse_slack_url(
            "https://myteam.slack.com/archives/C081VT5GLQH/p1770689887565249",
        );
        assert_eq!(
            result.unwrap(),
            SlackThread {
                channel_id: "C081VT5GLQH".to_string(),
                ts: "1770689887.565249".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_url_different_workspace() {
        let result =
            parse_slack_url("https://myteam.slack.com/archives/G012ABC3DEF/p1234567890123456");
        assert_eq!(
            result.unwrap(),
            SlackThread {
                channel_id: "G012ABC3DEF".to_string(),
                ts: "1234567890.123456".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_url_missing_p_prefix() {
        let result = parse_slack_url(
            "https://myteam.slack.com/archives/C081VT5GLQH/1770689887565249",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_url_too_few_segments() {
        let result = parse_slack_url("https://myteam.slack.com/archives/C081VT5GLQH");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_url_no_archives() {
        let result = parse_slack_url("https://myteam.slack.com/messages/C081VT5GLQH/p1770689887565249");
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_timestamp() {
        assert_eq!(
            convert_timestamp("p1770689887565249").unwrap(),
            "1770689887.565249"
        );
    }

    #[test]
    fn test_convert_timestamp_short() {
        assert!(convert_timestamp("p123").is_err());
    }
}
