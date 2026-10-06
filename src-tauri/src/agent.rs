use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

const OUTPUT_LIMIT: usize = 40_000;
const READ_CAP_BYTES: u64 = 512 * 1024;
const MAX_FILE_BYTES: u64 = 5 * 1024 * 1024;
const COMMAND_TIMEOUT_SECS: &str = "60";
const SEARCH_TIMEOUT_SECS: &str = "30";
const MAX_TOKENS: u32 = 8192;

// ---------- IPC shapes ----------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolFn {
    pub name: String,
    pub arguments: String,
}

fn default_tool_type() -> String {
    "function".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type", default = "default_tool_type")]
    pub kind: String,
    pub function: ToolFn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMsg {
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatReply {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub ok: bool,
    pub output: String,
}

impl ToolOutput {
    pub fn error(msg: &str) -> Self {
        ToolOutput { ok: false, output: clip(msg.to_string()) }
    }
}

#[derive(Debug, Clone)]
pub struct StreamDelta {
    pub content: Option<String>,
    pub reasoning: Option<String>,
}

/// Shared cancellation registry so the frontend can stop a streaming run.
#[derive(Default)]
pub struct RunCtl {
    cancelled: Mutex<HashSet<String>>,
}

impl RunCtl {
    pub fn cancel(&self, id: &str) {
        self.cancelled.lock().unwrap().insert(id.to_string());
    }

    pub fn is_cancelled(&self, id: &str) -> bool {
        self.cancelled.lock().unwrap().contains(id)
    }

    pub fn finish(&self, id: &str) {
        self.cancelled.lock().unwrap().remove(id);
    }
}

// ---------- workspace / sandbox ----------

pub fn home_dir() -> PathBuf {
    env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

pub fn workspace_dir() -> PathBuf {
    match env::var("OMMI_WORKSPACE") {
        Ok(w) if !w.trim().is_empty() => PathBuf::from(w),
        _ => home_dir(),
    }
}

fn allowed_roots() -> Vec<PathBuf> {
    let mut roots = vec![home_dir()];
    let ws = workspace_dir();
    if !roots.contains(&ws) {
        roots.push(ws);
    }
    roots
}

fn is_allowed(p: &Path) -> bool {
    allowed_roots().iter().any(|r| p.starts_with(r))
}

fn absolutize(p: &str) -> Result<PathBuf, String> {
    if p == "~" {
        return Ok(home_dir());
    }
    if let Some(rest) = p.strip_prefix("~/") {
        return Ok(home_dir().join(rest));
    }
    let pb = PathBuf::from(p);
    if pb.is_absolute() {
        Ok(pb)
    } else {
        Ok(workspace_dir().join(pb))
    }
}

/// Resolve a path that should already exist, rejecting anything outside HOME/workspace.
fn resolve_existing(p: &str) -> Result<PathBuf, String> {
    let abs = absolutize(p)?;
    let resolved = if abs.exists() {
        abs.canonicalize().map_err(|e| format!("cannot access {p}: {e}"))?
    } else {
        let parent = abs
            .parent()
            .filter(|x| !x.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("/"));
        if !parent.exists() {
            return Err(format!("path does not exist: {p}"));
        }
        let base = parent.canonicalize().map_err(|e| format!("cannot access {p}: {e}"))?;
        let name = abs.file_name().ok_or_else(|| format!("invalid path: {p}"))?;
        base.join(name)
    };
    if !is_allowed(&resolved) {
        return Err(format!("access denied: {p} is outside the allowed directories"));
    }
    Ok(resolved)
}

/// Resolve a path that may not exist yet (for writes), verifying the nearest
/// existing ancestor stays inside the sandbox and creating nothing.
fn resolve_write(p: &str) -> Result<PathBuf, String> {
    let abs = absolutize(p)?;
    if abs.exists() {
        let real = abs.canonicalize().map_err(|e| format!("cannot access {p}: {e}"))?;
        if !is_allowed(&real) {
            return Err(format!("access denied: {p} is outside the allowed directories"));
        }
        if real.is_dir() {
            return Err(format!("{p} is a directory, expected a file path"));
        }
        return Ok(real);
    }
    let mut probe = abs.clone();
    let mut missing: Vec<std::ffi::OsString> = Vec::new();
    while !probe.exists() {
        let name = probe
            .file_name()
            .map(|s| s.to_os_string())
            .ok_or_else(|| format!("invalid path: {p}"))?;
        missing.push(name);
        if !probe.pop() {
            return Err(format!("invalid path: {p}"));
        }
    }
    let mut real = probe
        .canonicalize()
        .map_err(|e| format!("cannot access parent of {p}: {e}"))?;
    if !is_allowed(&real) {
        return Err(format!("access denied: {p} is outside the allowed directories"));
    }
    for comp in missing.into_iter().rev() {
        real.push(comp);
    }
    Ok(real)
}

// ---------- helpers ----------

fn clip(mut s: String) -> String {
    if s.len() > OUTPUT_LIMIT {
        let mut end = OUTPUT_LIMIT;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        s.truncate(end);
        s.push_str("\n… [output truncated]");
    }
    s
}

fn ok(output: String) -> ToolOutput {
    ToolOutput { ok: true, output: clip(output) }
}

fn fail(output: String) -> ToolOutput {
    ToolOutput { ok: false, output: clip(output) }
}

fn arg_str(args: &Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("missing or invalid string argument '{key}'"))
}

fn arg_opt_str(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn arg_int(args: &Value, key: &str, default: u64) -> u64 {
    args.get(key)
        .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
        .unwrap_or(default)
}

// ---------- tool catalog ----------

pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub risk: &'static str,
    pub parameters: Value,
}

pub fn catalog() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "read_file",
            description: "Read a UTF-8 text file and return its content with line numbers. \
                          Files larger than 512KB are read partially from the start.",
            risk: "safe",
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path, absolute or relative to the working directory"},
                    "offset": {"type": "integer", "description": "1-based line number to start from (default 1)"},
                    "max_lines": {"type": "integer", "description": "Maximum number of lines to return (default 400)"}
                },
                "required": ["path"]
            }),
        },
        ToolDef {
            name: "list_dir",
            description: "List the entries of a directory (subdirectories first), with file sizes.",
            risk: "safe",
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Directory path, absolute or relative (default: working directory)"}
                }
            }),
        },
        ToolDef {
            name: "search_files",
            description: "Search file contents recursively with grep. Returns matching lines as \
                          'path:line:content'. Skips hidden directories (.git, .cache, .local, ...) \
                          as well as node_modules, target and dist. Pass an explicit path to search \
                          a specific directory.",
            risk: "safe",
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Text or regex pattern to search for"},
                    "path": {"type": "string", "description": "Directory to search in (default: working directory)"}
                },
                "required": ["query"]
            }),
        },
        ToolDef {
            name: "write_file",
            description: "Create or overwrite a text file with the given content. Creates parent \
                          directories when needed.",
            risk: "risky",
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path, absolute or relative to the working directory"},
                    "content": {"type": "string", "description": "Full new content of the file"}
                },
                "required": ["path", "content"]
            }),
        },
        ToolDef {
            name: "run_command",
            description: "Run a shell command with 'bash -c' in the working directory. 60s timeout. \
                          Returns stdout, stderr and the exit code.",
            risk: "risky",
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Shell command to execute"},
                    "cwd": {"type": "string", "description": "Working directory for the command (default: working directory)"}
                },
                "required": ["command"]
            }),
        },
    ]
}

/// Tool catalog for the frontend: includes the 'risk' field used for approval gating.
pub fn tools_catalog_json() -> Vec<Value> {
    catalog()
        .into_iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "risk": t.risk,
                "parameters": t.parameters
            })
        })
        .collect()
}

/// OpenAI-format tool list for the chat completion request (no 'risk' field).
fn tool_schemas() -> Value {
    Value::Array(
        catalog()
            .into_iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters
                    }
                })
            })
            .collect(),
    )
}

// ---------- tool implementations ----------

fn tool_read_file(args: &Value) -> ToolOutput {
    let path = match arg_str(args, "path") {
        Ok(p) => p,
        Err(e) => return fail(e),
    };
    let resolved = match resolve_existing(&path) {
        Ok(p) => p,
        Err(e) => return fail(e),
    };
    if resolved.is_dir() {
        return fail(format!("{path} is a directory; use list_dir instead"));
    }
    let meta = match fs::metadata(&resolved) {
        Ok(m) => m,
        Err(e) => return fail(format!("cannot read {path}: {e}")),
    };
    if meta.len() > MAX_FILE_BYTES {
        return fail(format!(
            "file is too large ({} bytes, limit {}); use offset/max_lines on a split file or search_files",
            meta.len(),
            MAX_FILE_BYTES
        ));
    }
    let file = match fs::File::open(&resolved) {
        Ok(f) => f,
        Err(e) => return fail(format!("cannot open {path}: {e}")),
    };
    let mut buf = Vec::new();
    if let Err(e) = file.take(READ_CAP_BYTES).read_to_end(&mut buf) {
        return fail(format!("cannot read {path}: {e}"));
    }
    let byte_truncated = meta.len() > buf.len() as u64;
    let text = String::from_utf8_lossy(&buf);
    let lines: Vec<&str> = text.lines().collect();
    let total = lines.len();
    let offset = arg_int(args, "offset", 1).max(1) as usize;
    let max_lines = arg_int(args, "max_lines", 400).max(1) as usize;
    let start = (offset - 1).min(total);
    let end = (start + max_lines).min(total);
    let mut out = String::new();
    for (i, line) in lines[start..end].iter().enumerate() {
        out.push_str(&format!("{:>6}\t{}\n", start + i + 1, line));
    }
    if out.is_empty() {
        out.push_str("(no lines returned — file is empty or offset is beyond the end)");
    }
    if byte_truncated {
        out.push_str("\n[note: file truncated at 512KB, showing the beginning only]");
    } else if end < total {
        out.push_str(&format!("\n[truncated: showing lines {}-{} of {}]", start + 1, end, total));
    }
    ok(out)
}

fn tool_list_dir(args: &Value) -> ToolOutput {
    let path = arg_opt_str(args, "path").unwrap_or_else(|| ".".to_string());
    let resolved = match resolve_existing(&path) {
        Ok(p) => p,
        Err(e) => return fail(e),
    };
    if !resolved.is_dir() {
        return fail(format!("{path} is not a directory"));
    }
    let max = arg_int(args, "max_entries", 200).max(1) as usize;
    let mut dirs: Vec<String> = Vec::new();
    let mut files: Vec<(String, u64)> = Vec::new();
    let rd = match fs::read_dir(&resolved) {
        Ok(rd) => rd,
        Err(e) => return fail(format!("cannot list {path}: {e}")),
    };
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if is_dir {
            dirs.push(name);
        } else {
            let len = entry.metadata().map(|m| m.len()).unwrap_or(0);
            files.push((name, len));
        }
    }
    dirs.sort();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let total = dirs.len() + files.len();
    let mut out = format!(
        "{} ({} entries{})\n",
        resolved.display(),
        total,
        if total > max { format!(", showing first {max}") } else { String::new() }
    );
    let mut shown = 0usize;
    for d in dirs.iter().take(max) {
        out.push_str(&format!("  {d}/\n"));
        shown += 1;
    }
    if shown < max {
        for (name, len) in files.iter().take(max - shown) {
            out.push_str(&format!("  {name}  {len} B\n"));
        }
    }
    if total == 0 {
        out.push_str("  (empty directory)\n");
    }
    ok(out)
}

fn tool_search_files(args: &Value) -> ToolOutput {
    let query = match arg_str(args, "query") {
        Ok(q) => q,
        Err(e) => return fail(e),
    };
    if query.trim().is_empty() {
        return fail("query must not be empty".to_string());
    }
    let path = arg_opt_str(args, "path").unwrap_or_else(|| ".".to_string());
    let dir = match resolve_existing(&path) {
        Ok(p) => p,
        Err(e) => return fail(e),
    };
    if !dir.is_dir() {
        return fail(format!("{path} is not a directory"));
    }
    let output = Command::new("timeout")
        .args(["-k", "3", SEARCH_TIMEOUT_SECS, "grep", "-rnI", "--binary-files=without-match"])
        .args(["--exclude-dir=.git", "--exclude-dir=node_modules", "--exclude-dir=target", "--exclude-dir=dist", "--exclude-dir=.*"])
        .arg("-e")
        .arg(&query)
        .arg(&dir)
        .output();
    let output = match output {
        Ok(o) => o,
        Err(e) => return fail(format!("search failed to start: {e}")),
    };
    let code = output.status.code();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if code == Some(1) {
        return ok(format!("No matches for '{query}' in {}", dir.display()));
    }
    if code == Some(124) {
        return fail(format!("search for '{query}' timed out after {SEARCH_TIMEOUT_SECS}s"));
    }
    if code == Some(2) && !stdout.trim().is_empty() {
        let mut lines: Vec<&str> = stdout.lines().collect();
        lines.truncate(200);
        let mut out = format!("Matches for '{query}' (up to 200):\n");
        for l in &lines {
            out.push_str(l);
            out.push('\n');
        }
        out.push_str(&format!("[note: some directories could not be read: {stderr}]"));
        return ok(out);
    }
    if code != Some(0) {
        return fail(format!("search failed: {stderr}"));
    }
    let mut lines: Vec<&str> = stdout.lines().collect();
    lines.truncate(200);
    let mut out = format!("Matches for '{query}' (up to 200):\n");
    for l in &lines {
        out.push_str(l);
        out.push('\n');
    }
    ok(out)
}

fn tool_write_file(args: &Value) -> ToolOutput {
    let path = match arg_str(args, "path") {
        Ok(p) => p,
        Err(e) => return fail(e),
    };
    let content = match args.get("content").and_then(|v| v.as_str()) {
        Some(c) => c.to_string(),
        None => return fail("missing or invalid string argument 'content'".to_string()),
    };
    let resolved = match resolve_write(&path) {
        Ok(p) => p,
        Err(e) => return fail(e),
    };
    if let Some(parent) = resolved.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return fail(format!("cannot create parent directories for {path}: {e}"));
        }
    }
    match fs::write(&resolved, content.as_bytes()) {
        Ok(()) => ok(format!("Wrote {} bytes to {}", content.len(), resolved.display())),
        Err(e) => fail(format!("cannot write {path}: {e}")),
    }
}

fn tool_run_command(args: &Value) -> ToolOutput {
    let command = match arg_str(args, "command") {
        Ok(c) => c,
        Err(e) => return fail(e),
    };
    let cwd = match arg_opt_str(args, "cwd") {
        Some(c) => match resolve_existing(&c) {
            Ok(p) => p,
            Err(e) => return fail(e),
        },
        None => workspace_dir(),
    };
    if !cwd.is_dir() {
        return fail(format!("working directory does not exist: {}", cwd.display()));
    }
    let output = Command::new("timeout")
        .args(["-k", "5", COMMAND_TIMEOUT_SECS, "bash", "-c", &command])
        .current_dir(&cwd)
        .output();
    let output = match output {
        Ok(o) => o,
        Err(e) => return fail(format!("failed to start command: {e}")),
    };
    let mut out = format!("$ {command}\n");
    out.push_str(&String::from_utf8_lossy(&output.stdout));
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !stderr.trim().is_empty() {
        out.push_str("\n[stderr]\n");
        out.push_str(&stderr);
    }
    match output.status.code() {
        Some(0) => ok(out),
        Some(124) => {
            out.push_str(&format!("\n[command timed out after {COMMAND_TIMEOUT_SECS}s]"));
            fail(out)
        }
        Some(code) => {
            out.push_str(&format!("\n[exit code: {code}]"));
            fail(out)
        }
        None => {
            out.push_str("\n[command terminated by signal]");
            fail(out)
        }
    }
}

/// Execute a tool by name. `args_json` is the JSON-encoded argument object.
/// Never panics: tool-level failures come back as `ok: false` with a message
/// the model can react to.
pub fn exec_tool(name: &str, args_json: &str) -> ToolOutput {
    let args: Value = match serde_json::from_str(args_json) {
        Ok(v) => v,
        Err(e) => return ToolOutput::error(&format!("invalid arguments JSON: {e}")),
    };
    if !args.is_object() {
        return ToolOutput::error("arguments must be a JSON object");
    }
    match name {
        "read_file" => tool_read_file(&args),
        "list_dir" => tool_list_dir(&args),
        "search_files" => tool_search_files(&args),
        "write_file" => tool_write_file(&args),
        "run_command" => tool_run_command(&args),
        other => ToolOutput::error(&format!("unknown tool: {other}")),
    }
}

// ---------- chat ----------

fn system_prompt() -> String {
    format!(
        "You are OmniCoder, an AI coding assistant running as a desktop application on the user's \
         computer. You have tools to read and write files and to run shell commands.\n\n\
         How to work:\n\
         - Use the tools to actually inspect the machine instead of guessing about file contents.\n\
         - Explore first: list_dir and read_file before editing; search_files to find code.\n\
         - Keep replies short: a sentence or two of explanation, then the result. Put code in \
         fenced code blocks.\n\
         - For edits, write the complete file content (write_file), never a partial diff.\n\
         - Never invent paths; verify them with list_dir first.\n\
         - If a tool reports an error or a denial, adapt and try another approach instead of \
         repeating the same call.\n\
         - Absolute paths are fine; relative paths resolve against the working directory.\n\n\
         Working directory: {}\n\
         Platform: Linux (bash available)",
        workspace_dir().display()
    )
}

// ---------- streaming chat completion ----------

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMsg],
    stream: bool,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Value>,
}

#[derive(Deserialize)]
struct SseChunk {
    #[serde(default)]
    choices: Vec<SseChoice>,
}

#[derive(Deserialize)]
struct SseChoice {
    #[serde(default)]
    delta: Option<Delta>,
}

#[derive(Deserialize)]
struct Delta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<ToolDelta>>,
}

#[derive(Deserialize)]
struct ToolDelta {
    #[serde(default)]
    index: usize,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<FnDelta>,
}

#[derive(Deserialize)]
struct FnDelta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Deserialize)]
struct FullResponse {
    #[serde(default)]
    choices: Vec<FullChoice>,
}

#[derive(Deserialize)]
struct FullChoice {
    message: ChatMsg,
}

fn gateway_base() -> String {
    env::var("OMNIROUTE_BASE_URL").unwrap_or_else(|_| "http://localhost:20128/v1".to_string())
}

fn gateway_key() -> String {
    env::var("OMNIROUTE_API_KEY").unwrap_or_default()
}

/// Stream one chat completion from the gateway.
///
/// Deltas are forwarded to `on_delta` as they arrive; `should_stop` is polled
/// between chunks so the frontend can cancel a run.
pub async fn chat_completion(
    messages: &[ChatMsg],
    model: &str,
    use_tools: bool,
    should_stop: &(dyn Fn() -> bool + Send + Sync),
    on_delta: &mut (dyn FnMut(&StreamDelta) + Send),
) -> Result<ChatReply, String> {
    if should_stop() {
        return Ok(ChatReply { content: String::new(), tool_calls: Vec::new(), cancelled: true });
    }

    let mut msgs: Vec<ChatMsg> = messages.to_vec();
    let first_is_system = msgs.first().map(|m| m.role == "system").unwrap_or(false);
    if use_tools && !first_is_system {
        msgs.insert(
            0,
            ChatMsg { role: "system".to_string(), content: Some(system_prompt()), tool_calls: None, tool_call_id: None },
        );
    }

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let mut http = client.post(format!("{}/chat/completions", gateway_base()));
    let key = gateway_key();
    if !key.is_empty() {
        http = http.header("Authorization", format!("Bearer {key}"));
    }
    let body = ChatRequest {
        model,
        messages: &msgs,
        stream: true,
        max_tokens: MAX_TOKENS,
        tools: if use_tools { Some(tool_schemas()) } else { None },
    };
    let res = http.json(&body).send().await.map_err(|e| format!("gateway request failed: {e}"))?;
    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(format!("gateway returned {status}: {}", clip(text)));
    }

    let is_stream = res
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.contains("text/event-stream"))
        .unwrap_or(false);

    if !is_stream {
        let text = res.text().await.map_err(|e| e.to_string())?;
        let full: FullResponse =
            serde_json::from_str(&text).map_err(|e| format!("unexpected gateway response: {e}"))?;
        let msg = full
            .choices
            .into_iter()
            .next()
            .map(|c| c.message)
            .ok_or_else(|| "gateway returned no choices".to_string())?;
        return Ok(ChatReply {
            content: msg.content.unwrap_or_default(),
            tool_calls: msg.tool_calls.unwrap_or_default(),
            cancelled: false,
        });
    }

    let mut res = res;
    let mut buf = String::new();
    let mut content = String::new();
    let mut reasoning = String::new();
    let mut acc: HashMap<usize, (String, String, String)> = HashMap::new();
    let mut cancelled = false;

    'stream: while let Some(chunk) = res.chunk().await.map_err(|e| format!("stream read failed: {e}"))? {
        if should_stop() {
            cancelled = true;
            break;
        }
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(nl) = buf.find('\n') {
            let line = buf[..nl].trim_end_matches('\r').to_string();
            buf.drain(..=nl);
            let trimmed = line.trim();
            let Some(data) = trimmed.strip_prefix("data:") else { continue };
            let data = data.trim_start();
            if data == "[DONE]" {
                break 'stream;
            }
            match serde_json::from_str::<SseChunk>(data) {
                Ok(parsed) => {
                    for choice in parsed.choices {
                        if let Some(delta) = choice.delta {
                            if let Some(r) = delta.reasoning_content.filter(|s| !s.is_empty()) {
                                reasoning.push_str(&r);
                                on_delta(&StreamDelta { content: None, reasoning: Some(r) });
                            }
                            if let Some(c) = delta.content.filter(|s| !s.is_empty()) {
                                content.push_str(&c);
                                on_delta(&StreamDelta { content: Some(c), reasoning: None });
                            }
                            if let Some(tds) = delta.tool_calls {
                                for td in tds {
                                    let entry = acc.entry(td.index).or_default();
                                    if let Some(id) = td.id {
                                        entry.0 = id;
                                    }
                                    if let Some(f) = td.function {
                                        if let Some(n) = f.name {
                                            entry.1 = n;
                                        }
                                        if let Some(a) = f.arguments {
                                            entry.2.push_str(&a);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    if data.contains("\"error\"") {
                        return Err(format!("gateway stream error: {}", clip(data.to_string())));
                    }
                }
            }
        }
    }

    if should_stop() {
        cancelled = true;
    }

    let mut indices: Vec<usize> = acc.keys().copied().collect();
    indices.sort_unstable();
    let tool_calls = indices
        .into_iter()
        .map(|i| {
            let (id, name, args) = &acc[&i];
            ToolCall {
                id: id.clone(),
                kind: "function".to_string(),
                function: ToolFn { name: name.clone(), arguments: args.clone() },
            }
        })
        .collect();

    Ok(ChatReply { content, tool_calls, cancelled })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_dir(name: &str) -> PathBuf {
        let dir = home_dir().join(format!(
            "omnicoder-agent-test-{}-{}",
            std::process::id(),
            name
        ));
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    #[test]
    fn tool_write_read_list_roundtrip() {
        let dir = base_dir("roundtrip");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("note.txt");

        let w = exec_tool(
            "write_file",
            &json!({"path": file.to_string_lossy(), "content": "line one\nline two\nline three\n"})
                .to_string(),
        );
        assert!(w.ok, "write failed: {}", w.output);
        assert!(w.output.contains("Wrote"), "{}", w.output);

        let r = exec_tool(
            "read_file",
            &json!({"path": file.to_string_lossy(), "offset": 2, "max_lines": 1}).to_string(),
        );
        assert!(r.ok, "read failed: {}", r.output);
        assert!(r.output.contains("line two"), "{}", r.output);
        assert!(!r.output.contains("line one"), "{}", r.output);

        let l = exec_tool("list_dir", &json!({"path": dir.to_string_lossy()}).to_string());
        assert!(l.ok, "list failed: {}", l.output);
        assert!(l.output.contains("note.txt"), "{}", l.output);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn sandbox_rejects_paths_outside_home() {
        let read = exec_tool("read_file", r#"{"path":"/etc/passwd"}"#);
        assert!(!read.ok, "read /etc/passwd should be denied");
        assert!(read.output.contains("access denied"), "{}", read.output);

        let write = exec_tool("write_file", r#"{"path":"/tmp/omnicoder-evil.txt","content":"x"}"#);
        assert!(!write.ok, "write outside home should be denied");
        assert!(write.output.contains("access denied"), "{}", write.output);

        let escape = exec_tool(
            "run_command",
            r#"{"command":"pwd","cwd":"/etc"}"#,
        );
        assert!(!escape.ok, "cwd outside home should be denied");
        assert!(escape.output.contains("access denied"), "{}", escape.output);

        let unknown = exec_tool("drop_database", "{}");
        assert!(!unknown.ok);
        assert!(unknown.output.contains("unknown tool"), "{}", unknown.output);

        let bad_json = exec_tool("read_file", "not json");
        assert!(!bad_json.ok);
        assert!(bad_json.output.contains("invalid arguments JSON"), "{}", bad_json.output);
    }

    #[test]
    fn run_command_reports_exit_codes() {
        let good = exec_tool("run_command", r#"{"command":"echo agent-ok-$((6*7))"}"#);
        assert!(good.ok, "{}", good.output);
        assert!(good.output.contains("agent-ok-42"), "{}", good.output);

        let bad = exec_tool("run_command", r#"{"command":"echo to-stderr >&2; exit 3"}"#);
        assert!(!bad.ok, "exit 3 should be failure");
        assert!(bad.output.contains("to-stderr"), "{}", bad.output);
        assert!(bad.output.contains("exit code: 3"), "{}", bad.output);
    }

    #[test]
    fn search_files_finds_matches() {
        let dir = base_dir("search");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("target.txt"), "alpha\nNEEDLE-8613\nomega\n").unwrap();
        let s = exec_tool(
            "search_files",
            &json!({"query": "NEEDLE-8613", "path": dir.to_string_lossy()}).to_string(),
        );
        assert!(s.ok, "{}", s.output);
        assert!(s.output.contains("target.txt"), "{}", s.output);
        let none = exec_tool(
            "search_files",
            &json!({"query": "zzz-no-such-string-zzz", "path": dir.to_string_lossy()}).to_string(),
        );
        assert!(none.ok, "{}", none.output);
        assert!(none.output.contains("No matches"), "{}", none.output);
        fs::remove_dir_all(&dir).ok();
    }

    fn user_msg(text: &str) -> Vec<ChatMsg> {
        vec![ChatMsg {
            role: "user".to_string(),
            content: Some(text.to_string()),
            tool_calls: None,
            tool_call_id: None,
        }]
    }

    fn test_model() -> String {
        env::var("OMMI_TEST_MODEL").unwrap_or_else(|_| "gemini/gemini-3.1-flash-lite".to_string())
    }

    #[tokio::test]
    async fn chat_streams_content_from_gateway() {
        let messages = user_msg("Reply with exactly this token and nothing else: HELLO-OMNI");
        let mut streamed = String::new();
        let reply = chat_completion(&messages, &test_model(), false, &|| false, &mut |d| {
            if let Some(c) = &d.content {
                streamed.push_str(c);
            }
        })
        .await
        .expect("chat_completion failed");
        assert!(!reply.cancelled);
        assert!(reply.tool_calls.is_empty());
        assert!(
            reply.content.contains("HELLO-OMNI") || streamed.contains("HELLO-OMNI"),
            "content: {:?} streamed: {:?}",
            reply.content,
            streamed
        );
        assert_eq!(reply.content, streamed, "streamed deltas must equal final content");
    }

    #[tokio::test]
    async fn chat_with_tools_returns_tool_calls() {
        let messages = user_msg(
            "Use the run_command tool to run the shell command `echo tool-ok`. \
             Do not answer with plain text first.",
        );
        let reply = chat_completion(&messages, &test_model(), true, &|| false, &mut |_: &_| {})
            .await
            .expect("chat_completion with tools failed");
        assert!(!reply.tool_calls.is_empty(), "expected tool calls, got: {}", reply.content);
        let tc = &reply.tool_calls[0];
        assert_eq!(tc.function.name, "run_command");
        assert_eq!(tc.kind, "function");
        assert!(!tc.id.is_empty());
        let args: Value = serde_json::from_str(&tc.function.arguments)
            .unwrap_or_else(|e| panic!("tool arguments not valid JSON ({e}): {}", tc.function.arguments));
        assert!(
            args.get("command").and_then(|c| c.as_str()).map(|c| c.contains("echo")).unwrap_or(false),
            "command arg: {args}"
        );
    }

    #[tokio::test]
    async fn chat_respects_cancellation() {
        let messages = user_msg("say hi");
        let reply = chat_completion(&messages, &test_model(), false, &|| true, &mut |_: &_| {})
            .await
            .expect("chat_completion cancelled early");
        assert!(reply.cancelled, "should_stop=true must cancel the run");
    }
}
