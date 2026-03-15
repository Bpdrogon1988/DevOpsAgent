use async_openai::{
    Client,
    types::{ChatCompletionRequestMessageArgs, CreateChatCompletionRequestArgs, Role},
};

pub async fn triage_error(error_log: &str, model: &str) -> Result<String, String> {
    let client = Client::new();

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(512u16)
        .model(model)
        .messages([
            ChatCompletionRequestMessageArgs::default()
                .role(Role::System)
                .content("You are an expert DevOps engineer who diagnoses deployment errors. Review the provided standard error logs and suggest a concise root cause and solution.")
                .build()
                .map_err(|err| err.to_string())?
                .into(),
            ChatCompletionRequestMessageArgs::default()
                .role(Role::User)
                .content(error_log)
                .build()
                .map_err(|err| err.to_string())?
                .into(),
        ])
        .build()
        .map_err(|err| err.to_string())?;

    let response = client
        .chat()
        .create(request)
        .await
        .map_err(|err| err.to_string())?;

    let solution = match response.choices.first() {
        Some(choice) => choice.message.content.clone(),
        None => "No solution provided by AI".to_string(),
    };

    Ok(solution)
}
