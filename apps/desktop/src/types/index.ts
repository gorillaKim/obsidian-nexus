export interface Project {
  id: string;
  name: string;
  vault_name: string | null;
  path: string;
  created_at: string | null;
  last_indexed_at: string | null;
}

export interface SearchResult {
  chunk_id: string;
  document_id: string;
  file_path: string;
  project_name: string;
  heading_path: string | null;
  snippet: string;
  score: number;
}

export interface ProjectStats {
  doc_count: number;
  chunk_count: number;
  pending_count: number;
}

export interface ProjectInfo {
  project: Project;
  stats: ProjectStats;
}

export interface DocItem {
  id: string;
  file_path: string;
  title: string | null;
}

export interface McpStatus {
  name: string;
  installed: boolean;
  registered: boolean;
}

export type Tab = "dashboard" | "search" | "projects" | "guide" | "settings";

export type SearchMode = "hybrid" | "keyword" | "vector";

// Agent / Chat types
export interface DetectedAgent {
  cli: "claude" | "gemini";
  path: string;
  version: string;
  authenticated: boolean;
  models: string[];
}

export interface SessionMeta {
  id: string;
  cli: "claude" | "gemini";
  model: string;
  name: string;
  project_id: string;
  created_at: number;
}

export interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  timestamp: number;
}

export type AgentStatus = "idle" | "generating" | "compacting" | "done" | "error";
