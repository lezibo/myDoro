use crate::state_machine::SharedState;
use crate::util::MutexExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct ResolvedSessionMeta {
    pub summary: String,
    pub cwd: String,
}

#[derive(Debug, Clone, Default)]
pub struct SessionDisplayMeta {
    pub agent_label: String,
    pub summary: String,
    pub project: String,
    pub short_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionChainBlock {
    pub kind: String,
    pub title: String,
    pub body: String,
}

pub(crate) fn display_agent_label(agent: &str) -> String {
    let normalized = agent.trim().to_ascii_lowercase();
    if normalized.contains("claude") {
        "Claude".into()
    } else if normalized.contains("codex") {
        "Codex".into()
    } else if normalized.contains("copilot") {
        "Copilot".into()
    } else if agent.trim().is_empty() {
        "Unknown".into()
    } else {
        agent.trim().into()
    }
}

pub(crate) fn short_session_id(session_id: &str) -> String {
    let stripped = session_id
        .strip_prefix("claude-monitor-")
        .or_else(|| session_id.strip_prefix("codex-"))
        .unwrap_or(session_id);
    let candidate = stripped
        .rsplit('-')
        .find(|part| part.len() >= 6)
        .unwrap_or(stripped);
    candidate.chars().take(6).collect()
}

pub(crate) fn codex_thread_url_for_session_id(session_id: &str) -> Option<String> {
    codex_thread_uuid_from_session_id(session_id).map(|uuid| format!("codex://threads/{uuid}"))
}

fn codex_thread_uuid_from_session_id(session_id: &str) -> Option<String> {
    let stripped = session_id.strip_prefix("codex-").unwrap_or(session_id);
    if is_uuid(stripped) {
        return Some(stripped.to_string());
    }

    let parts: Vec<&str> = stripped.rsplit('-').take(5).collect();
    if parts.len() != 5 {
        return None;
    }

    let candidate = format!(
        "{}-{}-{}-{}-{}",
        parts[4], parts[3], parts[2], parts[1], parts[0]
    );
    is_uuid(&candidate).then_some(candidate)
}

fn is_uuid(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    let lens = [8, 4, 4, 4, 12];
    parts.len() == lens.len()
        && parts
            .iter()
            .zip(lens)
            .all(|(part, len)| part.len() == len && part.chars().all(|c| c.is_ascii_hexdigit()))
}

pub(crate) fn project_name_from_cwd(cwd: &str) -> String {
    Path::new(cwd)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn clean_resume_summary(raw: &str) -> String {
    let mut fallback = String::new();

    for line in raw.lines() {
        let cleaned_line = clean_summary_line(line);
        if cleaned_line.is_empty() {
            continue;
        }
        if fallback.is_empty() {
            fallback = cleaned_line.clone();
        }
        if is_resume_noise(&cleaned_line) {
            continue;
        }
        return truncate_summary(&cleaned_line, 88);
    }

    truncate_summary(&fallback, 88)
}

pub(crate) fn ensure_session_display_meta(
    state: &SharedState,
    session_id: &str,
    fallback_agent: Option<&str>,
    fallback_cwd: Option<&str>,
) -> SessionDisplayMeta {
    let entry = {
        let sm = state.lock_or_recover();
        sm.sessions.get(session_id).cloned()
    };

    let mut agent_id = entry
        .as_ref()
        .map(|entry| entry.agent_id.trim().to_string())
        .filter(|agent| !agent.is_empty())
        .or_else(|| fallback_agent.map(|agent| agent.trim().to_string()))
        .unwrap_or_else(|| "claude-code".into());
    let mut summary = entry
        .as_ref()
        .map(|entry| entry.summary.clone())
        .unwrap_or_default();
    let mut cwd = entry
        .as_ref()
        .map(|entry| entry.cwd.clone())
        .unwrap_or_default();

    if summary.is_empty() || cwd.is_empty() {
        if let Some(resolved) = resolve_session_meta(session_id, &agent_id) {
            if summary.is_empty() && !resolved.summary.is_empty() {
                summary = resolved.summary.clone();
            }
            if cwd.is_empty() && !resolved.cwd.is_empty() {
                cwd = resolved.cwd.clone();
            }
            let mut sm = state.lock_or_recover();
            if let Some(entry) = sm.sessions.get_mut(session_id) {
                if entry.summary.is_empty() && !resolved.summary.is_empty() {
                    entry.summary = resolved.summary.clone();
                }
                if entry.cwd.is_empty() && !resolved.cwd.is_empty() {
                    entry.cwd = resolved.cwd.clone();
                }
                if entry.agent_id.trim().is_empty() {
                    entry.agent_id = agent_id.clone();
                }
            }
        }
    }

    if cwd.is_empty() {
        cwd = fallback_cwd.unwrap_or_default().to_string();
        if !cwd.is_empty() {
            let mut sm = state.lock_or_recover();
            if let Some(entry) = sm.sessions.get_mut(session_id) {
                if entry.cwd.is_empty() {
                    entry.cwd = cwd.clone();
                }
            }
        }
    }

    if agent_id.trim().is_empty() {
        agent_id = "claude-code".into();
    }

    SessionDisplayMeta {
        agent_label: display_agent_label(&agent_id),
        summary,
        project: project_name_from_cwd(&cwd),
        short_id: short_session_id(session_id),
    }
}

pub(crate) fn extract_tool_cwd(tool_input: &Value) -> Option<String> {
    const KEYS: &[&str] = &[
        "cwd",
        "workingDirectory",
        "working_directory",
        "dir",
        "path",
    ];
    KEYS.iter()
        .find_map(|key| tool_input.get(*key))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(crate) fn resolve_latest_session_summary(session_id: &str, agent: &str) -> Option<String> {
    let normalized = agent.trim().to_ascii_lowercase();
    if normalized.contains("codex") {
        resolve_latest_codex_session_summary(session_id)
    } else {
        resolve_latest_claude_session_summary(session_id)
    }
}

pub(crate) fn resolve_latest_session_chain(
    session_id: &str,
    agent: &str,
    max_blocks: usize,
) -> Vec<SessionChainBlock> {
    if max_blocks == 0 {
        return Vec::new();
    }

    let normalized = agent.trim().to_ascii_lowercase();
    if normalized.contains("codex") {
        resolve_latest_codex_session_chain(session_id, max_blocks)
    } else {
        Vec::new()
    }
}

fn resolve_session_meta(session_id: &str, agent: &str) -> Option<ResolvedSessionMeta> {
    let normalized = agent.trim().to_ascii_lowercase();
    if normalized.contains("codex") {
        resolve_codex_session_meta(session_id)
    } else {
        resolve_claude_session_meta(session_id)
    }
}

fn resolve_claude_session_meta(session_id: &str) -> Option<ResolvedSessionMeta> {
    let base = dirs::home_dir()?.join(".claude").join("projects");
    let hint = session_id
        .strip_prefix("claude-monitor-")
        .unwrap_or(session_id);
    let path = find_claude_session_file(&base, hint)?;
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut meta = ResolvedSessionMeta::default();

    for line in reader.lines().map_while(Result::ok) {
        let Ok(entry) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if meta.cwd.is_empty() {
            meta.cwd = entry
                .get("cwd")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
        }
        if meta.summary.is_empty() {
            if let Some(summary) = claude_entry_summary(&entry) {
                meta.summary = summary;
            }
        }
        if !meta.summary.is_empty() && !meta.cwd.is_empty() {
            break;
        }
    }

    if meta.summary.is_empty() && meta.cwd.is_empty() {
        None
    } else {
        Some(meta)
    }
}

fn resolve_latest_claude_session_summary(session_id: &str) -> Option<String> {
    let base = dirs::home_dir()?.join(".claude").join("projects");
    let hint = session_id
        .strip_prefix("claude-monitor-")
        .unwrap_or(session_id);
    let path = find_claude_session_file(&base, hint)?;
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut latest = String::new();
    for line in reader.lines().map_while(Result::ok) {
        let Ok(entry) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if let Some(summary) = claude_entry_summary(&entry) {
            latest = summary;
        }
    }

    (!latest.is_empty()).then_some(latest)
}

fn resolve_codex_session_meta(session_id: &str) -> Option<ResolvedSessionMeta> {
    let base = dirs::home_dir()?.join(".codex").join("sessions");
    let hint = session_id.strip_prefix("codex-").unwrap_or(session_id);
    let path = find_codex_session_file(&base, hint)?;
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut meta = ResolvedSessionMeta::default();

    for line in reader.lines().map_while(Result::ok) {
        let Ok(entry) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if meta.cwd.is_empty()
            && entry.get("type").and_then(|value| value.as_str()) == Some("session_meta")
        {
            meta.cwd = entry
                .get("payload")
                .and_then(|payload| payload.get("cwd"))
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
        }
        if meta.summary.is_empty() {
            if let Some(summary) = codex_entry_summary(&entry) {
                meta.summary = summary;
            }
        }
        if !meta.summary.is_empty() && !meta.cwd.is_empty() {
            break;
        }
    }

    if meta.summary.is_empty() && meta.cwd.is_empty() {
        None
    } else {
        Some(meta)
    }
}

fn resolve_latest_codex_session_summary(session_id: &str) -> Option<String> {
    let base = dirs::home_dir()?.join(".codex").join("sessions");
    let hint = session_id.strip_prefix("codex-").unwrap_or(session_id);
    let path = find_codex_session_file(&base, hint)?;
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut latest = String::new();
    for line in reader.lines().map_while(Result::ok) {
        let Ok(entry) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if let Some(summary) = codex_entry_summary(&entry) {
            latest = summary;
        }
    }

    (!latest.is_empty()).then_some(latest)
}

fn resolve_latest_codex_session_chain(
    session_id: &str,
    max_blocks: usize,
) -> Vec<SessionChainBlock> {
    let Some(base) = dirs::home_dir().map(|home| home.join(".codex").join("sessions")) else {
        return Vec::new();
    };
    let hint = session_id.strip_prefix("codex-").unwrap_or(session_id);
    let Some(path) = find_codex_session_file(&base, hint) else {
        return Vec::new();
    };
    let Ok(file) = File::open(path) else {
        return Vec::new();
    };
    let reader = BufReader::new(file);
    let mut blocks = Vec::new();
    let mut call_names: HashMap<String, String> = HashMap::new();

    for line in reader.lines().map_while(Result::ok) {
        let Ok(entry) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        append_codex_chain_block(&mut blocks, &mut call_names, &entry);
    }

    if blocks.len() > max_blocks {
        blocks.split_off(blocks.len() - max_blocks)
    } else {
        blocks
    }
}

fn append_codex_chain_block(
    blocks: &mut Vec<SessionChainBlock>,
    call_names: &mut HashMap<String, String>,
    entry: &Value,
) {
    let Some(top_type) = entry.get("type").and_then(|value| value.as_str()) else {
        return;
    };

    match top_type {
        "event_msg" => append_codex_event_msg_block(blocks, entry),
        "response_item" => append_codex_response_item_block(blocks, call_names, entry),
        _ => {}
    }
}

fn append_codex_event_msg_block(blocks: &mut Vec<SessionChainBlock>, entry: &Value) {
    let Some(payload) = entry.get("payload") else {
        return;
    };
    let Some(payload_type) = payload.get("type").and_then(|value| value.as_str()) else {
        return;
    };

    match payload_type {
        "user_message" => {
            let body = payload
                .get("message")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            if looks_like_system_prompt(body) {
                return;
            }
            push_session_chain_block(blocks, "user", "User", body);
        }
        "agent_message" => {
            let body = payload
                .get("message")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            push_session_chain_block(blocks, "assistant", "Codex", body);
        }
        "task_started" => push_session_chain_block(blocks, "event", "Task started", ""),
        "task_complete" => push_session_chain_block(blocks, "event", "Task complete", ""),
        "task_cancelled" => push_session_chain_block(blocks, "event", "Task cancelled", ""),
        _ => {}
    }
}

fn append_codex_response_item_block(
    blocks: &mut Vec<SessionChainBlock>,
    call_names: &mut HashMap<String, String>,
    entry: &Value,
) {
    let Some(payload) = entry.get("payload") else {
        return;
    };
    let Some(payload_type) = payload.get("type").and_then(|value| value.as_str()) else {
        return;
    };

    match payload_type {
        "reasoning" => {
            let body = codex_reasoning_summary(payload).unwrap_or_default();
            push_session_chain_block(blocks, "reasoning", "Thinking", &body);
        }
        "function_call" => {
            let name = payload
                .get("name")
                .and_then(|value| value.as_str())
                .unwrap_or("tool");
            if let Some(call_id) = payload.get("call_id").and_then(|value| value.as_str()) {
                call_names.insert(call_id.to_string(), name.to_string());
            }
            let body = payload
                .get("arguments")
                .and_then(format_codex_tool_arguments)
                .unwrap_or_default();
            push_session_chain_block(blocks, "tool_call", name, &body);
        }
        "function_call_output" => {
            let title = payload
                .get("call_id")
                .and_then(|value| value.as_str())
                .and_then(|call_id| call_names.get(call_id).cloned())
                .unwrap_or_else(|| "Tool result".to_string());
            let body = payload
                .get("output")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            push_session_chain_block(blocks, "tool_result", &title, body);
        }
        "message" => {
            let role = payload
                .get("role")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            let Some(body) = payload.get("content").and_then(extract_message_text) else {
                return;
            };
            match role {
                "assistant" => push_session_chain_block(blocks, "assistant", "Codex", &body),
                "user" => {
                    if !looks_like_system_prompt(&body) {
                        push_session_chain_block(blocks, "user", "User", &body);
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn push_session_chain_block(
    blocks: &mut Vec<SessionChainBlock>,
    kind: &str,
    title: &str,
    body: &str,
) {
    let block = SessionChainBlock {
        kind: kind.to_string(),
        title: title.to_string(),
        body: truncate_chain_body(body),
    };
    if block.body.is_empty() && block.kind != "reasoning" && block.kind != "event" {
        return;
    }
    if blocks.last() == Some(&block) {
        return;
    }
    blocks.push(block);
}

fn codex_reasoning_summary(payload: &Value) -> Option<String> {
    payload
        .get("summary")
        .and_then(extract_message_text)
        .or_else(|| payload.get("content").and_then(extract_message_text))
}

fn format_codex_tool_arguments(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        if let Ok(parsed) = serde_json::from_str::<Value>(text) {
            return format_codex_tool_arguments(&parsed);
        }
        return Some(text.to_string());
    }

    if let Some(command) = [
        "cmd",
        "command",
        "script",
        "query",
        "url",
        "path",
        "file_path",
        "ref_id",
    ]
    .iter()
    .find_map(|key| value.get(*key).and_then(|item| item.as_str()))
    {
        return Some(command.to_string());
    }

    serde_json::to_string(value).ok()
}

fn truncate_chain_body(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut lines = trimmed.lines();
    let visible_lines = lines
        .by_ref()
        .take(4)
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    let had_more_lines = lines.next().is_some();

    let mut chars = visible_lines.chars();
    let truncated: String = chars.by_ref().take(360).collect();
    if chars.next().is_some() || had_more_lines {
        format!("{}...", truncated.trim_end())
    } else {
        truncated
    }
}

fn claude_entry_summary(entry: &Value) -> Option<String> {
    if entry.get("type").and_then(|value| value.as_str()) == Some("user") {
        if let Some(content) = entry
            .get("message")
            .and_then(|message| message.get("content"))
            .and_then(extract_message_text)
        {
            return cleaned_non_system_summary(&content);
        }
    }

    entry
        .get("lastPrompt")
        .and_then(|value| value.as_str())
        .and_then(cleaned_non_system_summary)
}

fn codex_entry_summary(entry: &Value) -> Option<String> {
    if entry.get("type").and_then(|value| value.as_str()) == Some("event_msg")
        && entry
            .get("payload")
            .and_then(|payload| payload.get("type"))
            .and_then(|value| value.as_str())
            == Some("user_message")
    {
        if let Some(message) = entry
            .get("payload")
            .and_then(|payload| payload.get("message"))
            .and_then(|value| value.as_str())
        {
            return cleaned_non_system_summary(message);
        }
    }

    if entry.get("type").and_then(|value| value.as_str()) == Some("response_item")
        && entry
            .get("payload")
            .and_then(|payload| payload.get("type"))
            .and_then(|value| value.as_str())
            == Some("message")
        && entry
            .get("payload")
            .and_then(|payload| payload.get("role"))
            .and_then(|value| value.as_str())
            == Some("user")
    {
        if let Some(content) = entry
            .get("payload")
            .and_then(|payload| payload.get("content"))
            .and_then(extract_message_text)
        {
            return cleaned_non_system_summary(&content);
        }
    }

    None
}

fn cleaned_non_system_summary(raw: &str) -> Option<String> {
    if looks_like_system_prompt(raw) {
        return None;
    }
    let cleaned = clean_resume_summary(raw);
    (!cleaned.is_empty() && !looks_like_system_prompt(&cleaned)).then_some(cleaned)
}

fn extract_message_text(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }

    let parts = value.as_array()?;
    let mut chunks = Vec::new();
    for part in parts {
        if let Some(text) = part.get("text").and_then(|value| value.as_str()) {
            chunks.push(text.trim().to_string());
        }
    }
    if chunks.is_empty() {
        None
    } else {
        Some(chunks.join("\n"))
    }
}

fn find_claude_session_file(base: &Path, hint: &str) -> Option<PathBuf> {
    let mut matches = Vec::new();
    let dirs = std::fs::read_dir(base).ok()?;
    for dir in dirs.flatten() {
        let path = dir.path();
        if !path.is_dir() {
            continue;
        }
        let files = match std::fs::read_dir(&path) {
            Ok(files) => files,
            Err(_) => continue,
        };
        for file in files.flatten() {
            let path = file.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default();
            if stem == hint || stem.starts_with(hint) || hint.starts_with(stem) {
                matches.push(path);
            }
        }
    }
    newest_path(matches)
}

fn find_codex_session_file(base: &Path, hint: &str) -> Option<PathBuf> {
    let mut stack = vec![base.to_path_buf()];
    let mut matches = Vec::new();

    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default();
            if stem == hint || stem.contains(hint) {
                matches.push(path);
            }
        }
    }

    newest_path(matches)
}

fn newest_path(paths: Vec<PathBuf>) -> Option<PathBuf> {
    paths
        .into_iter()
        .max_by_key(|path| path.metadata().ok().and_then(|meta| meta.modified().ok()))
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn clean_summary_line(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let without_resume = trimmed
        .strip_prefix("/resume")
        .map(str::trim_start)
        .unwrap_or(trimmed);

    collapse_whitespace(without_resume)
}

fn is_resume_noise(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('#')
        || trimmed.starts_with('<')
        || trimmed.starts_with("```")
        || trimmed.starts_with("➜")
        || trimmed.starts_with('$')
        || trimmed.starts_with('%')
        || trimmed.starts_with('>')
        || trimmed.eq_ignore_ascii_case("user")
        || trimmed.eq_ignore_ascii_case("assistant")
        || trimmed.ends_with(".txt")
        || trimmed.ends_with(".json")
}

fn looks_like_system_prompt(text: &str) -> bool {
    text.starts_with("# AGENTS.md instructions")
        || text.starts_with("<environment_context>")
        || text.contains("<INSTRUCTIONS>")
}

fn truncate_summary(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{truncated}…")
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_resume_summary_from_shell_noise() {
        let raw = "➜ repo ls\nfoo.txt\n修复权限弹窗没有确认按钮的问题";
        assert_eq!(clean_resume_summary(raw), "修复权限弹窗没有确认按钮的问题");
    }

    #[test]
    fn strips_resume_prefix_from_inline_summary() {
        let raw = "/resume 修一下 permission request 的交互样式";
        assert_eq!(
            clean_resume_summary(raw),
            "修一下 permission request 的交互样式"
        );
    }

    #[test]
    fn skips_resume_only_line_and_uses_following_summary() {
        let raw = "/resume\n修一下 permission request 的交互样式";
        assert_eq!(
            clean_resume_summary(raw),
            "修一下 permission request 的交互样式"
        );
    }

    #[test]
    fn extracts_project_name_from_cwd() {
        assert_eq!(project_name_from_cwd("/Users/kinoko/Clyde"), "Clyde");
    }

    #[test]
    fn shortens_session_ids() {
        assert_eq!(short_session_id("claude-monitor-1f6449e7-6bcd"), "1f6449");
        assert_eq!(
            short_session_id("codex-rollout-2026-01-25-484ad236f061"),
            "484ad2"
        );
    }

    #[test]
    fn builds_codex_thread_url_from_rollout_filename() {
        assert_eq!(
            codex_thread_url_for_session_id(
                "codex-rollout-2026-06-22T10-43-25-019eed36-1821-7e70-b362-e1b5a169effc"
            )
            .as_deref(),
            Some("codex://threads/019eed36-1821-7e70-b362-e1b5a169effc")
        );
    }

    #[test]
    fn builds_codex_thread_url_from_uuid() {
        assert_eq!(
            codex_thread_url_for_session_id("codex-019eed36-1821-7e70-b362-e1b5a169effc")
                .as_deref(),
            Some("codex://threads/019eed36-1821-7e70-b362-e1b5a169effc")
        );
    }

    #[test]
    fn extracts_claude_user_entry_summary() {
        let entry = serde_json::json!({
            "type": "user",
            "message": {
                "content": [
                    { "type": "text", "text": "当前这轮需要读取配置" }
                ]
            }
        });

        assert_eq!(
            claude_entry_summary(&entry).as_deref(),
            Some("当前这轮需要读取配置")
        );
    }

    #[test]
    fn skips_codex_system_prompt_summary() {
        let system_entry = serde_json::json!({
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "user",
                "content": "# AGENTS.md instructions\n<INSTRUCTIONS>"
            }
        });
        let user_entry = serde_json::json!({
            "type": "event_msg",
            "payload": {
                "type": "user_message",
                "message": "请检查当前权限弹窗标题"
            }
        });

        assert_eq!(codex_entry_summary(&system_entry), None);
        assert_eq!(
            cleaned_non_system_summary(
                "# AGENTS.md instructions\n<INSTRUCTIONS>\n适用对象：`Codex`、`Kiro`、以及其他 agent"
            ),
            None
        );
        assert_eq!(
            codex_entry_summary(&user_entry).as_deref(),
            Some("请检查当前权限弹窗标题")
        );
    }

    #[test]
    fn skips_codex_system_prompt_blocks_for_session_chain() {
        let entry = serde_json::json!({
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "user",
                "content": "# AGENTS.md instructions\n<INSTRUCTIONS>\n适用对象：`Codex`、`Kiro`、以及其他 agent"
            }
        });
        let mut blocks = Vec::new();
        let mut call_names = HashMap::new();

        append_codex_chain_block(&mut blocks, &mut call_names, &entry);

        assert!(blocks.is_empty());
    }

    #[test]
    fn extracts_codex_tool_blocks_for_session_chain() {
        let call = serde_json::json!({
            "type": "response_item",
            "payload": {
                "type": "function_call",
                "name": "exec_command",
                "call_id": "call_1",
                "arguments": "{\"cmd\":\"cargo test\"}"
            }
        });
        let output = serde_json::json!({
            "type": "response_item",
            "payload": {
                "type": "function_call_output",
                "call_id": "call_1",
                "output": "Command: cargo test\nOutput:\nok"
            }
        });
        let mut blocks = Vec::new();
        let mut call_names = HashMap::new();

        append_codex_chain_block(&mut blocks, &mut call_names, &call);
        append_codex_chain_block(&mut blocks, &mut call_names, &output);

        assert_eq!(
            blocks,
            vec![
                SessionChainBlock {
                    kind: "tool_call".into(),
                    title: "exec_command".into(),
                    body: "cargo test".into(),
                },
                SessionChainBlock {
                    kind: "tool_result".into(),
                    title: "exec_command".into(),
                    body: "Command: cargo test\nOutput:\nok".into(),
                },
            ]
        );
    }

    #[test]
    fn extracts_codex_reasoning_and_assistant_text_blocks_for_session_chain() {
        let reasoning = serde_json::json!({
            "type": "response_item",
            "payload": {
                "type": "reasoning",
                "summary": [{"type": "summary_text", "text": "检查现有弹窗结构"}],
                "encrypted_content": "ignored"
            }
        });
        let message = serde_json::json!({
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "我会把链路展示在状态弹窗里。"}]
            }
        });
        let mut blocks = Vec::new();
        let mut call_names = HashMap::new();

        append_codex_chain_block(&mut blocks, &mut call_names, &reasoning);
        append_codex_chain_block(&mut blocks, &mut call_names, &message);

        assert_eq!(blocks[0].kind, "reasoning");
        assert_eq!(blocks[0].body, "检查现有弹窗结构");
        assert_eq!(blocks[1].kind, "assistant");
        assert_eq!(blocks[1].body, "我会把链路展示在状态弹窗里。");
    }
}
