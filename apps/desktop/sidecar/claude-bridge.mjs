#!/usr/bin/env node

/**
 * Claude Bridge — Agent SDK sidecar
 * V1 query() 기반 (systemPrompt + tools 정상 동작 확인됨)
 * stdin/stdout JSONL 프로토콜로 Tauri 앱과 통신
 * stdout = JSONL 프로토콜 전용 / stderr = 디버그 로그
 */

import { createInterface } from "readline";
import { existsSync } from "fs";
import { homedir } from "os";

/** Find the claude CLI binary, searching common locations */
function findClaudeBinary() {
  const candidates = [
    `${homedir()}/.local/bin/claude`,
    "/usr/local/bin/claude",
    "/opt/homebrew/bin/claude",
    "/usr/bin/claude",
  ];
  for (const p of candidates) {
    if (existsSync(p)) return p;
  }
  return "claude"; // fallback: hope it's in PATH
}

const log = (...args) => process.stderr.write(`[bridge] ${args.join(" ")}\n`);

/** Send a JSONL response to stdout */
function emit(response) {
  process.stdout.write(JSON.stringify(response) + "\n");
}

/** Active sessions: Map<sessionId, { options, sdkSessionId?, abort? }> */
const sessions = new Map();

let sdk = null;

async function loadSDK() {
  try {
    sdk = await import("@anthropic-ai/claude-agent-sdk");
    log("SDK loaded (V1 query mode)");
  } catch (err) {
    log("Failed to load SDK:", err.message);
    emit({
      type: "error",
      sessionId: "",
      code: "sdk_load_failed",
      message: `Failed to load @anthropic-ai/claude-agent-sdk: ${err.message}`,
      retryable: false,
    });
    process.exit(1);
  }
}

// === Request Handlers ===

async function handleStart(req) {
  const { sessionId, model, systemPrompt, mcpServers } = req;

  log(`Registering session ${sessionId} (model: ${model})`);

  const options = {
    model,
    systemPrompt,
    mcpServers: mcpServers || {},
    permissionMode: "bypassPermissions",
    pathToClaudeCodeExecutable: findClaudeBinary(),
  };

  sessions.set(sessionId, {
    options,
    sdkSessionId: null,
    abort: null,
  });

  emit({
    type: "init",
    sessionId,
    model,
    mcpServers: Object.keys(mcpServers || {}),
  });

  log(`Session ${sessionId} registered`);
}

async function handleMessage(req) {
  const { sessionId, content } = req;
  const entry = sessions.get(sessionId);

  if (!entry) {
    emit({
      type: "error",
      sessionId,
      code: "session_not_found",
      message: `Session ${sessionId} not found. Send a "start" request first.`,
      retryable: false,
    });
    return;
  }

  const abort = new AbortController();
  entry.abort = abort;

  try {
    const queryOpts = {
      ...entry.options,
      abortController: abort,
    };

    // Resume if we have a previous SDK session ID
    if (entry.sdkSessionId) {
      queryOpts.resume = entry.sdkSessionId;
    }

    log(`Sending message to session ${sessionId} (${entry.sdkSessionId ? "resume" : "new"})`);

    for await (const msg of sdk.query({ prompt: content, options: queryOpts })) {
      // Capture SDK session ID for future resume
      if (msg.type === "system" && msg.subtype === "init" && msg.session_id) {
        entry.sdkSessionId = msg.session_id;
        log(`SDK session ID captured: ${msg.session_id}`);
      }

      processSDKMessage(sessionId, msg);
    }
  } catch (err) {
    if (err.name === "AbortError") {
      log(`Session ${sessionId} cancelled`);
      return;
    }
    emit({
      type: "error",
      sessionId,
      code: "execution_error",
      message: err.message,
      retryable: true,
    });
  } finally {
    entry.abort = null;
  }
}

/** Convert SDK message to bridge protocol response */
function processSDKMessage(sessionId, msg) {
  switch (msg.type) {
    case "assistant": {
      if (!msg.message?.content) break;
      for (const block of msg.message.content) {
        if (block.type === "text" && block.text) {
          emit({ type: "text", sessionId, content: block.text, done: false });
        } else if (block.type === "thinking" && block.thinking) {
          emit({ type: "thought", sessionId, content: block.thinking });
        } else if (block.type === "tool_use") {
          emit({
            type: "tool_use",
            sessionId,
            toolName: block.name,
            input: block.input,
            status: "running",
          });
        } else if (block.type === "tool_result") {
          emit({
            type: "tool_use",
            sessionId,
            toolName: block.tool_use_id || "unknown",
            status: "done",
          });
        }
      }
      break;
    }

    case "result": {
      emit({
        type: "result",
        sessionId,
        content: msg.result || "",
        cost: msg.total_cost_usd,
        duration: msg.duration_ms,
        usage: msg.usage
          ? {
              input: msg.usage.input_tokens || 0,
              output: msg.usage.output_tokens || 0,
              cacheRead: msg.usage.cache_read_input_tokens,
              cacheCreation: msg.usage.cache_creation_input_tokens,
            }
          : undefined,
      });
      break;
    }

    // Skip system, user, hook, rate_limit messages
    default:
      break;
  }
}

function handleCancel(req) {
  const { sessionId } = req;
  const entry = sessions.get(sessionId);
  if (entry?.abort) {
    entry.abort.abort();
    log(`Cancelled session ${sessionId}`);
  }
}

function handleClose(req) {
  const { sessionId } = req;
  const entry = sessions.get(sessionId);
  if (entry) {
    if (entry.abort) entry.abort.abort();
    sessions.delete(sessionId);
    log(`Closed session ${sessionId}`);
  }
}

// === Main Loop ===

async function main() {
  await loadSDK();

  const rl = createInterface({ input: process.stdin });

  rl.on("line", async (line) => {
    const trimmed = line.trim();
    if (!trimmed) return;

    let req;
    try {
      req = JSON.parse(trimmed);
    } catch {
      log(`Invalid JSON: ${trimmed}`);
      return;
    }

    switch (req.type) {
      case "start":
        await handleStart(req);
        break;
      case "message":
        await handleMessage(req);
        break;
      case "cancel":
        handleCancel(req);
        break;
      case "close":
        handleClose(req);
        break;
      default:
        log(`Unknown request type: ${req.type}`);
    }
  });

  rl.on("close", () => {
    log("stdin closed, shutting down");
    for (const [id, entry] of sessions) {
      if (entry.abort) entry.abort.abort();
    }
    sessions.clear();
    process.exit(0);
  });

  log("Claude bridge ready");
}

main().catch((err) => {
  log(`Fatal error: ${err.message}`);
  process.exit(1);
});
