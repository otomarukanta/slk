mod error;
mod json;
mod message;
mod slack_api;
mod url;

use error::SlkError;

fn parse_args(args: Vec<String>) -> Result<String, SlkError> {
    args.into_iter()
        .nth(1)
        .ok_or(SlkError::from("usage: slk <slack-thread-url>"))
}

fn format_messages(messages: &[message::SlackMessage]) -> String {
    messages
        .iter()
        .map(|m| {
            format!(
                "{} [{}] {}",
                message::format_unix_ts(&m.ts),
                m.user,
                m.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn run(args: Vec<String>) -> Result<String, SlkError> {
    let url_str = parse_args(args)?;
    let token = std::env::var("SLACK_TOKEN")
        .map_err(|_| SlkError::from("SLACK_TOKEN environment variable is not set"))?;
    let thread = url::parse_slack_url(&url_str)?;
    let raw_json = slack_api::fetch_thread_replies(&thread.channel_id, &thread.ts, &token)?;
    let json_value = json::parse(&raw_json)?;
    let messages = message::extract_messages(&json_value)?;
    Ok(format_messages(&messages))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match run(args) {
        Ok(output) => println!("{}", output),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_valid() {
        let args = vec![
            "slk".to_string(),
            "https://myteam.slack.com/archives/C081VT5GLQH/p1770689887565249".to_string(),
        ];
        let result = parse_args(args).unwrap();
        assert_eq!(
            result,
            "https://myteam.slack.com/archives/C081VT5GLQH/p1770689887565249"
        );
    }

    #[test]
    fn test_parse_args_no_url() {
        let args = vec!["slk".to_string()];
        assert!(parse_args(args).is_err());
    }

    #[test]
    fn test_format_messages() {
        let messages = vec![
            message::SlackMessage {
                user: "U081R4ZS5E2".to_string(),
                text: "Hello, this is a thread".to_string(),
                ts: "1770689887.565249".to_string(),
            },
            message::SlackMessage {
                user: "U092X3AB7F1".to_string(),
                text: "Great thread!".to_string(),
                ts: "1770689900.000100".to_string(),
            },
        ];
        let output = format_messages(&messages);
        assert_eq!(
            output,
            "2026-02-10 02:18:07 [U081R4ZS5E2] Hello, this is a thread\n2026-02-10 02:18:20 [U092X3AB7F1] Great thread!"
        );
    }

    #[test]
    fn test_format_messages_empty() {
        let messages: Vec<message::SlackMessage> = vec![];
        assert_eq!(format_messages(&messages), "");
    }
}
