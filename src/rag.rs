use async_openai::{
    types::{CreateChatCompletionRequestArgs, ChatCompletionRequestMessageArgs, Role},
    Client,
};

pub async fn triage_error(error_log: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new(); // Automatically picks up OPENAI_API_KEY from environment

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(512u16)
        .model("gpt-4o-mini")
        .messages([
            ChatCompletionRequestMessageArgs::default()
                .role(Role::System)
                .content("You are an expert DevOps engineer who diagnoses deployment errors. Review the provided standard error logs and suggest a concise root cause and solution.")
                .build()?
                .into(),
            ChatCompletionRequestMessageArgs::default()
                .role(Role::User)
                .content(error_log)
                .build()?
                .into(),
        ])
        .build()?;

    let response = client.chat().create(request).await?;
    
    let solution = match response.choices.first() {
        Some(choice) => choice.message.content.clone(),
        None => "No solution provided by AI".to_string(),
    };

    Ok(solution)
}
