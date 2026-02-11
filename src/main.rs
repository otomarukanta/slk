mod config;
mod error;
mod json;
mod message;
mod oauth;
mod slack_api;
mod url;

use std::collections::HashMap;

use error::SlkError;

enum Command {
    Login,
    ShowThread { url: String },
}

fn parse_args(args: Vec<String>) -> Result<Command, SlkError> {
    let arg = args
        .into_iter()
        .nth(1)
        .ok_or(SlkError::from("usage: slk <slack-thread-url>\n       slk login"))?;

    if arg == "login" {
        Ok(Command::Login)
    } else {
        Ok(Command::ShowThread { url: arg })
    }
}

fn resolve_token() -> Result<String, SlkError> {
    if let Ok(token) = std::env::var("SLACK_TOKEN") {
        if !token.is_empty() {
            return Ok(token);
        }
    }
    if let Some(token) = config::load_token()? {
        return Ok(token);
    }
    Err(SlkError::from(
        "no Slack token found. Set SLACK_TOKEN or run: slk login",
    ))
}

fn format_messages(
    messages: &[message::SlackMessage],
    user_names: &HashMap<String, String>,
) -> String {
    messages
        .iter()
        .map(|m| {
            let display = match user_names.get(&m.user) {
                Some(name) => format!("@{}", name),
                None => m.user.clone(),
            };
            format!(
                "{} {} {}",
                message::format_unix_ts(&m.ts),
                display,
                m.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn resolve_user_names(
    messages: &[message::SlackMessage],
    token: &str,
) -> Result<HashMap<String, String>, SlkError> {
    let unique_ids: std::collections::HashSet<&str> = messages
        .iter()
        .map(|m| m.user.as_str())
        .filter(|id| id.starts_with('U'))
        .collect();

    let mut names = HashMap::new();
    for id in unique_ids {
        let raw = slack_api::fetch_user_info(id, token)?;
        let json_val = json::parse(&raw)?;
        let name = message::resolve_user_name(&json_val)?;
        names.insert(id.to_string(), name);
    }
    Ok(names)
}

fn run_login() -> Result<String, SlkError> {
    let (client_id, client_secret) = config::load_client_credentials()?;
    let token = oauth::run_oauth_flow(&client_id, &client_secret)?;
    let path = config::save_token(&token)?;
    Ok(format!("Token saved to {}", path.display()))
}

fn run_show_thread(url_str: &str) -> Result<String, SlkError> {
    let token = resolve_token()?;
    let thread = url::parse_slack_url(url_str)?;
    let raw_json = slack_api::fetch_thread_replies(&thread.channel_id, &thread.ts, &token)?;
    let json_value = json::parse(&raw_json)?;
    let messages = message::extract_messages(&json_value)?;
    let user_names = resolve_user_names(&messages, &token)?;
    Ok(format_messages(&messages, &user_names))
}

fn run(args: Vec<String>) -> Result<String, SlkError> {
    match parse_args(args)? {
        Command::Login => run_login(),
        Command::ShowThread { url } => run_show_thread(&url),
    }
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
    fn test_parse_args_valid_url() {
        let args = vec![
            "slk".to_string(),
            "https://myteam.slack.com/archives/C081VT5GLQH/p1770689887565249".to_string(),
        ];
        let result = parse_args(args).unwrap();
        match result {
            Command::ShowThread { url } => assert_eq!(
                url,
                "https://myteam.slack.com/archives/C081VT5GLQH/p1770689887565249"
            ),
            _ => panic!("expected ShowThread"),
        }
    }

    #[test]
    fn test_parse_args_login() {
        let args = vec!["slk".to_string(), "login".to_string()];
        let result = parse_args(args).unwrap();
        assert!(matches!(result, Command::Login));
    }

    #[test]
    fn test_parse_args_no_args() {
        let args = vec!["slk".to_string()];
        assert!(parse_args(args).is_err());
    }

    #[test]
    fn test_format_messages_with_resolved_names() {
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
        let mut user_names = HashMap::new();
        user_names.insert("U081R4ZS5E2".to_string(), "kanta".to_string());
        user_names.insert("U092X3AB7F1".to_string(), "taro".to_string());
        let output = format_messages(&messages, &user_names);
        assert_eq!(
            output,
            "2026-02-10 02:18:07 @kanta Hello, this is a thread\n2026-02-10 02:18:20 @taro Great thread!"
        );
    }

    #[test]
    fn test_format_messages_unresolved_fallback() {
        let messages = vec![message::SlackMessage {
            user: "U081R4ZS5E2".to_string(),
            text: "Hello".to_string(),
            ts: "1770689887.565249".to_string(),
        }];
        let user_names = HashMap::new();
        let output = format_messages(&messages, &user_names);
        assert_eq!(output, "2026-02-10 02:18:07 U081R4ZS5E2 Hello");
    }

    #[test]
    fn test_format_messages_empty() {
        let messages: Vec<message::SlackMessage> = vec![];
        let user_names = HashMap::new();
        assert_eq!(format_messages(&messages, &user_names), "");
    }
}
