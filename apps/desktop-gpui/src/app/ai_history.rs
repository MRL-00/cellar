use super::ai::{AiMessage, AiRole};

pub(super) fn clean_history(messages: &[AiMessage]) -> Vec<(AiRole, String)> {
    let mut history = Vec::new();
    for message in messages {
        if message.role == AiRole::Model && message.error {
            if history
                .last()
                .is_some_and(|(role, _)| *role == AiRole::User)
            {
                history.pop();
            }
            continue;
        }
        history.push((
            message.role,
            message
                .api_content
                .as_ref()
                .unwrap_or(&message.content)
                .clone(),
        ));
    }
    history
}

#[cfg(test)]
mod tests {
    use super::clean_history;
    use crate::app::ai::{AiMessage, AiRole};

    #[test]
    fn failed_ai_turns_do_not_poison_provider_history() {
        let messages = vec![
            AiMessage {
                role: AiRole::User,
                content: "visible".into(),
                api_content: Some("context + visible".into()),
                error: false,
                total_tokens: None,
                model: None,
            },
            AiMessage {
                role: AiRole::Model,
                content: "failed".into(),
                api_content: None,
                error: true,
                total_tokens: None,
                model: None,
            },
        ];
        assert!(clean_history(&messages).is_empty());
    }
}
