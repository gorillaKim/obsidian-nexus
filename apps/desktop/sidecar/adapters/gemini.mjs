/**
 * Gemini CLI Adapter
 *
 * Gemini CLI를 child_process.spawn으로 실행하여 stream-json 출력을 파싱합니다.
 * - --output-format stream-json: 구조화된 JSONL 이벤트 스트림
 * - --approval-mode yolo: 헤드리스 실행 시 도구 자동 승인
 * - GEMINI_SYSTEM_MD: 시스템 프롬프트 주입 (temp 파일 경유)
 * - MCP 연동: ~/.gemini/settings.json 또는 --allowed-mcp-server-names 플래그
 */

import { spawn } from "child_process";
import { dirname } from "path";
import { writeFileSync, unlinkSync, mkdtempSync } from "fs";
import { tmpdir } from "os";
import { join } from "path";
import { randomUUID } from "crypto";

const log = (...args) => process.stderr.write(`[gemini] ${args.join(" ")}\n`);

// 헤드리스 실행 타임아웃 (5분)
const QUERY_TIMEOUT_MS = 5 * 60 * 1000;

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

    // [CRITICAL fix] symlink attack 방지: mkdtempSync으로 예측 불가 디렉토리 생성
    let systemMdDir = null;
    let systemMdPath = null;
    if (systemPrompt) {
      systemMdDir = mkdtempSync(join(tmpdir(), "nexus-gemini-"));
      systemMdPath = join(systemMdDir, "system.md");
      // 0o600: 현재 사용자만 읽기/쓰기 가능
      writeFileSync(systemMdPath, systemPrompt, { encoding: "utf8", mode: 0o600 });
    }

    const cleanup = () => {
      if (systemMdPath) {
        try { unlinkSync(systemMdPath); } catch (e) { log("temp file cleanup failed:", e.message); }
        systemMdPath = null;
      }
    };

    // Gemini CLI 헤드리스 플래그:
    //   -p: non-interactive 모드
    //   --output-format stream-json: JSONL 이벤트 스트림
    //   --approval-mode yolo: 도구 자동 승인 (헤드리스 필수)
    const args = [
      "-p", prompt,
      "--output-format", "stream-json",
      "--approval-mode", "yolo",
    ];
    if (model) args.push("-m", model);

    log(`Spawning gemini (model: ${model || "default"})`);

    // nvm 경로의 Node.js 스크립트 shebang 해석을 위해 CLI 디렉토리를 PATH에 추가
    const cliDir = dirname(cliPath);
    const enrichedPath = cliDir + ":" + (process.env.PATH || "");

    const env = {
      ...process.env,
      PATH: enrichedPath,
      ...(systemMdPath ? { GEMINI_SYSTEM_MD: systemMdPath } : {}),
    };

    const child = spawn(cliPath, args, {
      stdio: ["ignore", "pipe", "pipe"],
      env,
    });

    // entry.abort 호환: handleCancel이 entry.abort.abort()를 호출
    entry.abort = { abort: () => child.kill("SIGTERM") };

    let fullText = "";
    let lineBuffer = "";

    child.stdout.on("data", (chunk) => {
      lineBuffer += chunk.toString();
      const lines = lineBuffer.split("\n");
      lineBuffer = lines.pop(); // 마지막 불완전 줄 보관

      for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) continue;

        let event;
        try {
          event = JSON.parse(trimmed);
        } catch {
          // JSONL 아닌 줄(예: 디버그 출력)은 텍스트로 처리
          fullText += trimmed + "\n";
          emit({ type: "text", sessionId, content: trimmed + "\n", done: false });
          continue;
        }

        switch (event.type) {
          case "message":
            if (event.role === "model" || event.role === "assistant") {
              const text = event.content ?? "";
              fullText += text;
              emit({ type: "text", sessionId, content: text, done: false });
            }
            break;
          case "tool_use":
            log(`tool_use: ${event.tool_name}`);
            break;
          case "tool_result":
            log(`tool_result: ${event.tool_name}`);
            break;
          case "result":
            log(`result: response length=${fullText.length}`);
            break;
          case "error":
            log(`stream error: ${JSON.stringify(event)}`);
            break;
          default:
            break;
        }
      }
    });

    child.stderr.on("data", (chunk) => {
      log(`stderr: ${chunk.toString().trim()}`);
    });

    await new Promise((resolve) => {
      // [HIGH fix] close + error 모두 발생 시 double-emit 방지
      let settled = false;

      // [MEDIUM fix] 타임아웃: 5분 초과 시 강제 종료
      const timer = setTimeout(() => {
        if (!settled) {
          log("query timeout — killing gemini");
          child.kill("SIGTERM");
        }
      }, QUERY_TIMEOUT_MS);

      const finish = (emitFn) => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        entry.abort = null;
        cleanup();
        emitFn();
        resolve();
      };

      child.on("close", (code) => {
        finish(() => {
          if (code === 0 || code === null) {
            emit({ type: "result", sessionId, content: fullText });
          } else if (code === 53) {
            emit({
              type: "error",
              sessionId,
              code: "turn_limit",
              message: "Gemini 턴 한도를 초과했습니다. 대화를 나눠서 진행해 주세요.",
              retryable: false,
            });
          } else {
            emit({
              type: "error",
              sessionId,
              code: "execution_error",
              message: `gemini CLI가 종료 코드 ${code}로 실패했습니다.`,
              retryable: true,
            });
          }
        });
      });

      child.on("error", (err) => {
        log(`spawn error: ${err.message}`);
        finish(() => {
          emit({
            type: "error",
            sessionId,
            code: "spawn_error",
            message: `Gemini CLI 실행 실패: ${err.message}`,
            retryable: false,
          });
        });
      });
    });
  }
}
