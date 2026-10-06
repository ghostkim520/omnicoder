use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Command;
use std::env;
use tauri::Manager;

mod agent;

const DEFAULT_MODEL: &str = "gemini/gemini-3.1-flash-lite";

#[derive(Serialize, Deserialize)]
struct AIRequest {
    model: String,
    messages: Vec<AIMessage>,
    stream: bool,
    max_tokens: u32,
}

#[derive(Serialize, Deserialize)]
struct AIMessage {
    role: String,
    #[serde(default)]
    content: String,
}

#[derive(Serialize, Deserialize)]
struct AIResponse {
    choices: Vec<AIChoice>,
}

#[derive(Serialize, Deserialize)]
struct AIChoice {
    message: AIMessage,
}

#[derive(Serialize, Deserialize)]
pub struct CodeResponse {
    pub code: Option<String>,
    pub error: Option<String>,
}

#[tauri::command]
async fn generate_code(prompt: String, model: Option<String>, use_local: bool, max_tokens: Option<u32>) -> Result<CodeResponse, String> {
    let _ = use_local;
    let base_url = env::var("OMNIROUTE_BASE_URL").unwrap_or_else(|_| "http://localhost:20128/v1".to_string());
    let api_key = env::var("OMNIROUTE_API_KEY").unwrap_or_else(|_| "".to_string());
    let selected_model = model.unwrap_or_else(|| env::var("OMMI_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string()));
    let max_tokens = max_tokens.unwrap_or(4096);

    let client = reqwest::Client::new();
    let res = client.post(format!("{}/chat/completions", base_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&AIRequest {
            model: selected_model,
            stream: false,
            max_tokens,
            messages: vec![
                AIMessage {
                    role: "system".to_string(),
                    content: "You are an expert coding assistant. Provide only the code, no explanation.".to_string(),
                },
                AIMessage {
                    role: "user".to_string(),
                    content: prompt,
                },
            ],
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if res.status().is_success() {
        let ai_res: AIResponse = res.json().await.map_err(|e| e.to_string())?;
        match ai_res.choices.into_iter().next() {
            Some(choice) if !choice.message.content.trim().is_empty() => {
                Ok(CodeResponse { code: Some(choice.message.content), error: None })
            }
            Some(_) => Ok(CodeResponse {
                code: None,
                error: Some("Model returned an empty response. Try a larger max_tokens or another model.".to_string()),
            }),
            None => Ok(CodeResponse { code: None, error: Some("Model returned no choices.".to_string()) }),
        }
    } else {
        let err_text = res.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Ok(CodeResponse { code: None, error: Some(err_text) })
    }
}

#[tauri::command]
async fn debug_code(code: String) -> Result<String, String> {
    let base_url = env::var("OMNIROUTE_BASE_URL").unwrap_or_else(|_| "http://localhost:20128/v1".to_string());
    let api_key = env::var("OMNIROUTE_API_KEY").unwrap_or_else(|_| "".to_string());
    let model = env::var("OMMI_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

    let client = reqwest::Client::new();
    let res = client.post(format!("{}/chat/completions", base_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&AIRequest {
            model,
            stream: false,
            max_tokens: 4096,
            messages: vec![
                AIMessage {
                    role: "system".to_string(),
                    content: "Debug the following code. Point out errors and suggest fixes.".to_string(),
                },
                AIMessage {
                    role: "user".to_string(),
                    content: code,
                },
            ],
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if res.status().is_success() {
        let ai_res: AIResponse = res.json().await.map_err(|e| e.to_string())?;
        let text = ai_res.choices.into_iter().next()
            .map(|c| c.message.content)
            .filter(|c| !c.trim().is_empty())
            .unwrap_or_else(|| "Model returned an empty response.".to_string());
        Ok(text)
    } else {
        Err(res.text().await.unwrap_or_else(|_| "Unknown error".to_string()))
    }
}

#[tauri::command]
fn run_command(command: String) -> Result<String, String> {
    let output = Command::new("bash")
        .arg("-c")
        .arg(&command)
        .output()
        .map_err(|e| e.to_string())?;

    let mut result = String::from_utf8_lossy(&output.stdout).to_string();
    result.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(result)
}

#[tauri::command]
fn git_status() -> Result<String, String> {
    let output = Command::new("git")
        .arg("status")
        .output()
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[tauri::command]
fn git_commit(message: String) -> Result<(), String> {
    let output = Command::new("git")
        .arg("commit")
        .arg("-am")
        .arg(message)
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
fn git_push() -> Result<(), String> {
    let output = Command::new("git")
        .arg("push")
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
async fn list_models() -> Result<Vec<String>, String> {
    let base_url = env::var("OMNIROUTE_BASE_URL").unwrap_or_else(|_| "http://localhost:20128/v1".to_string());
    let client = reqwest::Client::new();
    let res = client.get(format!("{}/models", base_url))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    #[derive(Deserialize)]
    struct ModelList { data: Vec<Model> }
    #[derive(Deserialize)]
    struct Model { id: String }

    let models: ModelList = res.json().await.map_err(|e| e.to_string())?;
    let mut ids: Vec<String> = models.data.into_iter().map(|m| m.id).collect();
    ids.retain(|id| {
        id.contains("mistral")
            || id.contains("codestral")
            || (id.contains("gemini") && id.contains("flash"))
    });
    ids.sort();
    ids.dedup();
    for fallback in ["gemini/gemini-3.1-flash-lite", "gemini/gemini-3.5-flash"] {
        if !ids.contains(&fallback.to_string()) {
            ids.push(fallback.to_string());
        }
    }
    Ok(ids)
}

#[tauri::command]
fn gateway_status() -> Result<bool, String> {
    let base_url = env::var("OMNIROUTE_BASE_URL").unwrap_or_else(|_| "http://localhost:20128/v1".to_string());
    let res = Command::new("curl")
        .arg("-s")
        .arg("-o")
        .arg("/dev/null")
        .arg("-w")
        .arg("%{http_code}")
        .arg(format!("{}/models", base_url))
        .output();

    match res {
        Ok(output) => {
            let code = String::from_utf8_lossy(&output.stdout);
            Ok(code == "200")
        },
        _ => Ok(false)
    }
}

/// Stream one chat completion. Emits `chat-chunk` events
/// `{ run_id, content, reasoning }` while streaming; returns the full reply.
#[tauri::command]
async fn chat(
    app: tauri::AppHandle,
    messages: Vec<agent::ChatMsg>,
    model: Option<String>,
    use_tools: bool,
    run_id: String,
) -> Result<agent::ChatReply, String> {
    use tauri::Emitter;

    let model = model
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| env::var("OMMI_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string()));
    let ctl = app.state::<agent::RunCtl>();
    let stop_ctl = ctl;
    let stop_id = run_id.clone();
    let should_stop = move || stop_ctl.is_cancelled(&stop_id);

    let emit_app = app.clone();
    let emit_id = run_id.clone();
    let mut on_delta = move |d: &agent::StreamDelta| {
        let _ = emit_app.emit(
            "chat-chunk",
            serde_json::json!({
                "run_id": emit_id,
                "content": d.content,
                "reasoning": d.reasoning,
            }),
        );
    };

    let result = agent::chat_completion(&messages, &model, use_tools, &should_stop, &mut on_delta).await;
    app.state::<agent::RunCtl>().finish(&run_id);
    result
}

#[tauri::command]
fn chat_cancel(app: tauri::AppHandle, run_id: String) {
    app.state::<agent::RunCtl>().cancel(&run_id);
}

/// Execute a desktop tool (file/shell access). Runs on a blocking thread so
/// long shell commands do not stall the async runtime.
#[tauri::command]
async fn exec_tool(name: String, args: String) -> agent::ToolOutput {
    tauri::async_runtime::spawn_blocking(move || agent::exec_tool(&name, &args))
        .await
        .unwrap_or_else(|e| agent::ToolOutput::error(&format!("tool execution failed: {e}")))
}

/// Tool catalog for the frontend, including the `risk` field used for
/// approval gating (`safe` tools run automatically, `risky` ones ask first).
#[tauri::command]
fn list_tools() -> Vec<Value> {
    agent::tools_catalog_json()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .manage(agent::RunCtl::default())
    .invoke_handler(tauri::generate_handler![
        generate_code,
        debug_code,
        run_command,
        git_status,
        git_commit,
        git_push,
        list_models,
        gateway_status,
        chat,
        chat_cancel,
        exec_tool,
        list_tools
    ])
    .setup(|app| {
      app.handle().plugin(
        tauri_plugin_log::Builder::default()
          .level(log::LevelFilter::Info)
          .build(),
      )?;
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while building tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn gateway_reachable() {
        assert!(gateway_status().unwrap_or(false), "OmniRoute gateway not reachable");
    }

    #[tokio::test]
    async fn models_include_mistral() {
        let models = list_models().await.expect("list_models failed");
        assert!(!models.is_empty(), "empty model list");
        assert!(models.iter().any(|m| m.contains("mistral")), "no mistral models: {models:?}");
    }

    #[tokio::test]
    async fn generate_code_via_gateway() {
        let model = env::var("OMMI_TEST_MODEL").unwrap_or_else(|_| "gemini/gemini-3.1-flash-lite".to_string());
        let mut last = String::new();
        for attempt in 1..=3 {
            match generate_code(
                "Write a Python function add(a, b) that returns the sum of two numbers. Code only.".to_string(),
                Some(model.clone()),
                false,
                Some(4096),
            ).await {
                Ok(res) => {
                    if let Some(code) = res.code {
                        assert!(!code.trim().is_empty());
                        println!("attempt {attempt} ok: {}", &code[..code.len().min(80)]);
                        return;
                    }
                    last = res.error.unwrap_or_else(|| "empty response".to_string());
                }
                Err(e) => last = e,
            }
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
        panic!("generate_code never succeeded via {model}: {last}");
    }
}
