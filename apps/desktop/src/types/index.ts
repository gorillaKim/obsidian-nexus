export type CliType = "claude" | "gemini";

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

export interface PopularDoc {
  id: string;
  file_path: string;
  title: string;
  project_id: string;
  project_name: string;
  view_count: number;
  backlink_count: number;
  score: number;
  last_modified: string | null;
}

export interface TopProject {
  id: string;
  name: string;
  activity: number;
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
  cli: CliType;
  path: string;
  version: string;
  authenticated: boolean;
  models: string[];
}

export interface SessionMeta {
  id: string;
  cli: CliType;
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

export interface ComponentStatus {
  installed: boolean;
  detail: string | null;
}

export interface CliAgentStatus {
  cli: CliType;
  installed: boolean;
  path: string | null;
  version: string | null;
  authenticated: boolean;
  failure_reason: string | null;
}

export interface CliDiagnostics {
  cli: string;
  which_result: string;
  direct_exec_stdout: string;
  direct_exec_stderr: string;
  direct_exec_exit: string;
  shell_exec_stdout: string;
  shell_exec_stderr: string;
  shell_exec_exit: string;
  shell_used: string;
  nvm_path: string;
  nvm_exec_stdout: string;
  nvm_exec_exit: string;
  find_cli_path_result: string;
}

export interface SystemStatus {
  mcp_binary: ComponentStatus;
  obs_nexus_binary: ComponentStatus;
  mcp_registrations: McpStatus[];
  cli_agents: CliAgentStatus[];
  ollama: ComponentStatus;
  obsidian: ComponentStatus;
}
