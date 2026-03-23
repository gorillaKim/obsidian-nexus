/**
 * Claude Agent SDK Adapter
 * Uses @anthropic-ai/claude-agent-sdk (query() API with MCP support)
 */

const log = (...args) => process.stderr.write(`[claude] ${args.join(" ")}\n`);

export class ClaudeAdapter {
  #sdk = null;

  async loadSDK() {
    try {
      this.#sdk = await import("@anthropic-ai/claude-agent-sdk");
      log("Claude Agent SDK loaded");
    } catch (err) {
      log("Failed to load Claude Agent SDK:", err.message);
      throw err;
    }
  }

  /** Build Claude-specific session options from the start request */
  buildOptions(req) {
    const { model, systemPrompt, mcpServers, cliPath } = req;
    return {
      model,
      systemPrompt,
      mcpServers: mcpServers || {},
      permissionMode: "bypassPermissions",
      allowDangerouslySkipPermissions: true,
      canUseTool: async (toolName, _input, opts) => {
        const allowed =
          toolName.startsWith("nexus_") ||
          ["Read", "LS", "Glob", "Grep", "WebSearch", "WebFetch"].includes(toolName);
        if (allowed) {
          return { behavior: "allow", updatedPermissions: opts.suggestions };
        }
        return { behavior: "deny", message: `Tool '${toolName}' is not permitted in this context.` };
      },
      pathToClaudeCodeExecutable: cliPath,
    };
  }

  /** Execute a message for a session, emitting bridge protocol events */
  async query(sessionId, prompt, entry, emit) {
    const abort = new AbortController();
    entry.abort = abort;

    const queryOpts = {
      ...entry.options,
      abortController: abort,
    };

    if (entry.sdkSessionId) {
      queryOpts.resume = entry.sdkSessionId;
    }

    log(`Sending message to session ${sessionId} (${entry.sdkSessionId ? "resume" : "new"})`);

    try {
      for await (const msg of this.#sdk.query({ prompt, options: queryOpts })) {
        if (msg.type === "system" && msg.subtype === "init" && msg.session_id) {
          entry.sdkSessionId = msg.session_id;
          log(`SDK session ID captured: ${msg.session_id}`);
        }
        this.#processMessage(sessionId, msg, emit);
      }
    } catch (err) {
      if (err.name === "AbortError") {
        log(`Session ${sessionId} cancelled`);
        emit({ type: "cancelled", sessionId });
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

  /** Map Claude SDK message to bridge protocol */
  #processMessage(sessionId, msg, emit) {
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

      default:
        break;
    }
  }
}
