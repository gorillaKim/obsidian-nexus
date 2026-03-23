/**
 * Gemini CLI Adapter
 *
 * Gemini CLI를 child_process.spawn으로 실행하여 stdout을 스트리밍합니다.
 * MCP 연동은 향후 ~/.gemini/settings.json 주입 방식으로 확장 예정.
 */

import { spawn } from "child_process";

const log = (...args) => process.stderr.write(`[gemini] ${args.join(" ")}\n`);

export class GeminiAdapter {
  async loadSDK() {
    log("Gemini adapter ready");
  }

  buildOptions(req) {
    return {
      cliPath: req.cliPath,
      model: req.model,
      systemPrompt: req.systemPrompt || "",
    };
  }

  async query(sessionId, prompt, entry, emit) {
    const { cliPath, model, systemPrompt } = entry.options;

    // System prompt을 사용자 메시지 앞에 붙여 전달
    const fullPrompt = systemPrompt
      ? `${systemPrompt}\n\n${prompt}`
      : prompt;

    // Gemini CLI: gemini -p "<prompt>" [-m <model>]
    const args = ["-p", fullPrompt];
    if (model) args.push("-m", model);

    log(`Spawning gemini (model: ${model || "default"})`);

    const child = spawn(cliPath, args, {
      stdio: ["ignore", "pipe", "pipe"],
      env: { ...process.env },
    });

    // entry.abort 호환: handleCancel이 entry.abort.abort()를 호출
    entry.abort = { abort: () => child.kill("SIGTERM") };

    let fullText = "";

    child.stdout.on("data", (chunk) => {
      const text = chunk.toString();
      fullText += text;
      emit({ type: "text", sessionId, content: text, done: false });
    });

    child.stderr.on("data", (chunk) => {
      log(`stderr: ${chunk.toString().trim()}`);
    });

    await new Promise((resolve) => {
      child.on("close", (code) => {
        entry.abort = null;
        if (code === 0 || code === null) {
          emit({ type: "result", sessionId, content: fullText });
        } else {
          emit({
            type: "error",
            sessionId,
            code: "execution_error",
            message: `gemini CLI exited with code ${code}`,
            retryable: true,
          });
        }
        resolve();
      });

      child.on("error", (err) => {
        entry.abort = null;
        log(`spawn error: ${err.message}`);
        emit({
          type: "error",
          sessionId,
          code: "spawn_error",
          message: `Failed to spawn gemini CLI: ${err.message}`,
          retryable: false,
        });
        resolve();
      });
    });
  }
}
