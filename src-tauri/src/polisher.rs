use crate::prompt::{SYSTEM_PROMPT, TRANSFORM_SYSTEM_PROMPT};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct Msg {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct Req {
    model: String,
    messages: Vec<Msg>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Deserialize)]
struct Resp {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: MsgContent,
}

#[derive(Deserialize)]
struct MsgContent {
    content: String,
}

pub async fn polish_text(text: &str, api_key: &str) -> Result<String, String> {
    let client = Client::new();
    let body = Req {
        model: "openai/gpt-4o-mini".to_string(),
        messages: vec![
            Msg { role: "system".to_string(), content: SYSTEM_PROMPT.to_string() },
            Msg { role: "user".to_string(), content: text.to_string() },
        ],
        max_tokens: 2048,
        temperature: 0.3,
    };

    let resp = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body_text = resp.text().await.unwrap_or_default();
        return Err(format!("API {}: {}", status, body_text));
    }

    let data: Resp = resp.json().await.map_err(|e| e.to_string())?;
    data.choices
        .into_iter()
        .next()
        .map(|c| c.message.content.trim().to_string())
        .ok_or_else(|| "Empty response from API".to_string())
}

pub async fn transform_text(text: &str, instruction: &str, api_key: &str) -> Result<String, String> {
    let client = Client::new();
    let user_content = format!("Instruction: {}\n\nText:\n{}", instruction, text);
    let body = Req {
        model: "openai/gpt-4o-mini".to_string(),
        messages: vec![
            Msg { role: "system".to_string(), content: TRANSFORM_SYSTEM_PROMPT.to_string() },
            Msg { role: "user".to_string(), content: user_content },
        ],
        max_tokens: 2048,
        temperature: 0.4,
    };

    let resp = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body_text = resp.text().await.unwrap_or_default();
        return Err(format!("API {}: {}", status, body_text));
    }

    let data: Resp = resp.json().await.map_err(|e| e.to_string())?;
    data.choices
        .into_iter()
        .next()
        .map(|c| c.message.content.trim().to_string())
        .ok_or_else(|| "Empty response from API".to_string())
}
