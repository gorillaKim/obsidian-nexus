/**
 * Bridge Protocol — JSONL 요청/응답 타입 정의
 * Rust ↔ Node.js sidecar 통신 프로토콜
 * Claude/Gemini 모두 이 프로토콜을 따름
 */

// === 요청 (Rust → Node stdin) ===

export type BridgeRequest =
  | StartRequest
  | MessageRequest
  | CancelRequest
  | CloseRequest;

export interface StartRequest {
  type: "start";
  sessionId: string;
  model: string;
  systemPrompt: string;
  mcpServers: Record<string, McpServerConfig>;
  allowedTools?: string[];
}

export interface MessageRequest {
  type: "message";
  sessionId: string;
  content: string;
}

export interface CancelRequest {
  type: "cancel";
  sessionId: string;
}

export interface CloseRequest {
  type: "close";
  sessionId: string;
}

export interface McpServerConfig {
  command: string;
  args?: string[];
  env?: Record<string, string>;
}

// === 응답 (Node stdout → Rust) ===

export type BridgeResponse =
  | InitResponse
  | ThoughtResponse
  | ToolUseResponse
  | TextResponse
  | ResultResponse
  | ErrorResponse;

export interface InitResponse {
  type: "init";
  sessionId: string;
  model: string;
  mcpServers?: string[];
}

export interface ThoughtResponse {
  type: "thought";
  sessionId: string;
  content: string;
}

export interface ToolUseResponse {
  type: "tool_use";
  sessionId: string;
  toolName: string;
  input?: Record<string, unknown>;
  status: "running" | "done";
}

export interface TextResponse {
  type: "text";
  sessionId: string;
  content: string;
  done: boolean;
}

export interface ResultResponse {
  type: "result";
  sessionId: string;
  content: string;
  cost?: number;
  duration?: number;
  usage?: {
    input: number;
    output: number;
    cacheRead?: number;
    cacheCreation?: number;
  };
}

export interface ErrorResponse {
  type: "error";
  sessionId: string;
  code: string;
  message: string;
  retryable: boolean;
}
