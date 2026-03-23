/**
 * Gemini CLI Adapter (stub)
 *
 * Gemini CLI supports MCP via ~/.gemini/settings.json.
 * Future implementation: spawn gemini CLI as child process, inject nexus MCP server config,
 * pipe stdout/stderr, parse streaming output.
 *
 * When ready:
 *   const child = spawn(cliPath, ["--model", model, prompt, ...])
 *   child.stdout.on("data", chunk => parseAndEmit(chunk))
 *   abortController.signal.addEventListener("abort", () => child.kill())
 */

const log = (...args) => process.stderr.write(`[gemini] ${args.join(" ")}\n`);

export class GeminiAdapter {
  async loadSDK() {
    log("Gemini adapter loaded (stub — not yet implemented)");
  }

  buildOptions(req) {
    return { cliPath: req.cliPath, model: req.model };
  }

  async query(sessionId, _prompt, _entry, emit) {
    log(`Session ${sessionId}: Gemini support not yet implemented`);
    emit({
      type: "error",
      sessionId,
      code: "not_supported",
      message: "Gemini CLI support is not yet implemented",
      retryable: false,
    });
  }
}
