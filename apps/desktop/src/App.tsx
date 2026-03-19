import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";

interface Project {
  id: string;
  name: string;
  vault_name: string | null;
  path: string;
  created_at: string | null;
  last_indexed_at: string | null;
}

interface SearchResult {
  chunk_id: string;
  document_id: string;
  file_path: string;
  project_name: string;
  heading_path: string | null;
  snippet: string;
  score: number;
}

interface ProjectStats {
  doc_count: number;
  chunk_count: number;
  pending_count: number;
}

interface ProjectInfo {
  project: Project;
  stats: ProjectStats;
}

interface DocItem {
  id: string;
  file_path: string;
  title: string | null;
}

type Tab = "dashboard" | "search" | "projects" | "guide" | "settings";

interface McpStatus {
  name: string;
  installed: boolean;
  registered: boolean;
}

function SettingsTab() {
  const [mcpStatuses, setMcpStatuses] = useState<McpStatus[]>([]);
  const [registering, setRegistering] = useState<string | null>(null);

  const loadStatus = useCallback(async () => {
    try {
      const statuses = await invoke<McpStatus[]>("mcp_status");
      setMcpStatuses(statuses);
    } catch (e) {
      console.error("Failed to load MCP status", e);
    }
  }, []);

  useEffect(() => { loadStatus(); }, [loadStatus]);

  const handleRegister = async (name: string) => {
    setRegistering(name);
    try {
      await invoke("mcp_register", { name });
      await loadStatus();
    } catch (e) {
      console.error("Failed to register MCP", e);
    }
    setRegistering(null);
  };

  return (
    <div className="max-w-2xl mx-auto">
      <h2 className="text-lg font-bold mb-4" style={{ color: "var(--accent)" }}>설정</h2>

      <div className="p-4 rounded-lg mb-4" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
        <h3 className="font-medium mb-3">MCP 서버 자동 등록</h3>
        <p className="text-sm opacity-70 mb-4">
          AI 도구에 Obsidian Nexus MCP 서버를 등록하면, 에이전트가 볼트 문서를 직접 검색하고 읽을 수 있습니다.
        </p>

        <div className="space-y-2">
          {mcpStatuses.map((s) => (
            <div key={s.name} className="flex items-center justify-between p-3 rounded"
              style={{ background: "var(--bg-primary)", border: `1px solid var(--border)` }}>
              <div className="flex items-center gap-3">
                <span className="text-sm font-medium">{s.name}</span>
                {!s.installed && (
                  <span className="text-xs px-2 py-0.5 rounded opacity-50"
                    style={{ background: "var(--bg-secondary)" }}>미설치</span>
                )}
                {s.installed && s.registered && (
                  <span className="text-xs px-2 py-0.5 rounded"
                    style={{ background: "#9ece6a22", color: "#9ece6a" }}>등록됨</span>
                )}
                {s.installed && !s.registered && (
                  <span className="text-xs px-2 py-0.5 rounded"
                    style={{ background: "#f7768e22", color: "#f7768e" }}>미등록</span>
                )}
              </div>
              {s.installed && !s.registered && (
                <button
                  onClick={() => handleRegister(s.name)}
                  disabled={registering === s.name}
                  className="px-3 py-1 rounded text-xs cursor-pointer"
                  style={{ background: "var(--accent)", color: "#1a1b26" }}>
                  {registering === s.name ? "등록 중..." : "등록"}
                </button>
              )}
            </div>
          ))}
          {mcpStatuses.length === 0 && (
            <p className="text-sm opacity-50">MCP 대상 도구를 확인 중...</p>
          )}
        </div>
      </div>

      <div className="p-4 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
        <h3 className="font-medium mb-2">지원 도구</h3>
        <ul className="text-sm opacity-70 space-y-1">
          <li>• <strong>Claude Desktop</strong> — Anthropic 데스크톱 앱</li>
          <li>• <strong>Claude Code</strong> — CLI 기반 코딩 에이전트</li>
          <li>• <strong>Gemini CLI</strong> — Google Gemini CLI 도구</li>
        </ul>
        <p className="text-xs opacity-50 mt-3">
          등록 후 해당 AI 도구를 재시작해야 MCP 서버가 인식됩니다.
        </p>
      </div>
    </div>
  );
}

type SearchMode = "hybrid" | "keyword" | "vector";

function App() {
  const [tab, setTab] = useState<Tab>("dashboard");
  const [projects, setProjects] = useState<Project[]>([]);
  const [projectInfos, setProjectInfos] = useState<Map<string, ProjectInfo>>(new Map());
  const [query, setQuery] = useState("");
  const [selectedProject, setSelectedProject] = useState<string>("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [indexing, setIndexing] = useState<Set<string>>(new Set());
  const [adding, setAdding] = useState(false);
  const [syncing, setSyncing] = useState<Set<string>>(new Set());
  const [viewingDoc, setViewingDoc] = useState<{ projectId: string; filePath: string; content: string } | null>(null);
  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(new Set());
  const [projectDocs, setProjectDocs] = useState<Map<string, DocItem[]>>(new Map());
  const [searchMode, setSearchMode] = useState<SearchMode>("hybrid");
  const [expandedResults, setExpandedResults] = useState<Set<string>>(new Set());
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set());
  const [showSettings, setShowSettings] = useState(false);
  const [tagFilter, setTagFilter] = useState("");
  const [hybridWeight, setHybridWeight] = useState(0.7);
  const [minVectorScore, setMinVectorScore] = useState(0.65);

  const loadProjects = useCallback(async () => {
    try {
      const list = await invoke<Project[]>("list_projects");
      setProjects(list);
      const infos = new Map<string, ProjectInfo>();
      for (const p of list) {
        try {
          const info = await invoke<ProjectInfo>("project_info", { projectId: p.id });
          infos.set(p.id, info);
        } catch { /* skip */ }
      }
      setProjectInfos(infos);
    } catch (e) {
      console.error(e);
    }
  }, []);

  useEffect(() => { loadProjects(); }, [loadProjects]);

  const handleSearch = async () => {
    if (!query.trim()) return;
    setSearching(true);
    setViewingDoc(null);
    try {
      const parsedTags = tagFilter.trim()
        ? tagFilter.split(",").map((t) => t.trim()).filter((t) => t)
        : undefined;
      const res = await invoke<SearchResult[]>("search_documents", {
        query,
        projectId: selectedProject || null,
        limit: 20,
        mode: searchMode,
        hybridWeight: searchMode === "hybrid" ? hybridWeight : undefined,
        minVectorScore: searchMode !== "keyword" ? minVectorScore : undefined,
        tags: parsedTags,
      });
      setResults(res);
    } catch (e) { console.error(e); }
    setSearching(false);
  };

  const handleIndex = async (projectId: string) => {
    setIndexing((prev) => new Set(prev).add(projectId));
    try {
      await invoke("index_project", { projectId });
      await loadProjects();
    } catch (e) { console.error(e); }
    setIndexing((prev) => { const n = new Set(prev); n.delete(projectId); return n; });
  };

  const handleAddVault = async () => {
    try {
      const selected = await open({ directory: true, title: "볼트 폴더 선택" });
      if (!selected) return;
      setAdding(true);
      const folderPath = selected as string;

      // Auto-detect Obsidian vaults (.obsidian folder) and register them
      const result = await invoke<{ added: number; projects: unknown[] }>("auto_add_vaults", { path: folderPath });
      if (result.added === 0) {
        // No vaults found — fall back to registering the folder directly
        const folderName = folderPath.split("/").pop() || "untitled";
        await invoke("add_project", { name: folderName, path: folderPath });
      }
      await loadProjects();
    } catch (e) { console.error(e); }
    setAdding(false);
  };

  const handleSync = async (projectId: string) => {
    setSyncing((prev) => new Set(prev).add(projectId));
    try {
      await invoke("sync_vault_config", { projectId });
      await loadProjects();
    } catch (e) { console.error(e); }
    setSyncing((prev) => { const n = new Set(prev); n.delete(projectId); return n; });
  };

  const handleRemoveProject = async (projectId: string) => {
    try {
      await invoke("remove_project", { projectId });
      await loadProjects();
    } catch (e) { console.error(e); }
  };

  const openFile = async (project: Project, filePath: string) => {
    try { await invoke("open_file", { projectId: project.id, filePath }); }
    catch (e) { console.error(e); }
  };

  const viewDocument = async (projectId: string, filePath: string) => {
    try {
      const content = await invoke<string>("get_document", { projectId, filePath });
      setViewingDoc({ projectId, filePath, content });
    } catch (e) { console.error(e); }
  };


  const toggleProject = async (projectId: string) => {
    const next = new Set(expandedProjects);
    if (next.has(projectId)) {
      next.delete(projectId);
    } else {
      next.add(projectId);
      if (!projectDocs.has(projectId)) {
        try {
          const docs = await invoke<DocItem[]>("list_documents", { projectId });
          setProjectDocs((prev) => new Map(prev).set(projectId, docs));
        } catch (e) { console.error(e); }
      }
    }
    setExpandedProjects(next);
  };

  // Group docs by folder for tree view
  const buildTree = (docs: DocItem[]) => {
    const folders: Map<string, DocItem[]> = new Map();
    const rootDocs: DocItem[] = [];
    for (const doc of docs) {
      const parts = doc.file_path.split("/");
      if (parts.length > 1) {
        const folder = parts.slice(0, -1).join("/");
        if (!folders.has(folder)) folders.set(folder, []);
        folders.get(folder)!.push(doc);
      } else {
        rootDocs.push(doc);
      }
    }
    return { folders, rootDocs };
  };

  const totalDocs = Array.from(projectInfos.values()).reduce((sum, i) => sum + i.stats.doc_count, 0);

  const tabLabels: Record<Tab, string> = {
    dashboard: "대시보드",
    search: "검색",
    projects: "프로젝트",
    guide: "가이드",
    settings: "설정",
  };

  return (
    <div className="min-h-screen flex flex-col" style={{ background: "var(--bg-primary)" }}>
      {/* 헤더 */}
      <header className="px-6 py-4 flex items-center gap-4 border-b sticky top-0 z-50" style={{ borderColor: "var(--border)", background: "var(--bg-primary)" }}>
        <h1 className="text-xl font-bold" style={{ color: "var(--accent)" }}>Obsidian Nexus</h1>
        <nav className="flex gap-2 ml-auto">
          {(["dashboard", "search", "projects", "guide", "settings"] as Tab[]).map((t) => (
            <button
              key={t}
              onClick={() => { setTab(t); setViewingDoc(null); }}
              className={`px-3 py-1 rounded text-sm ${tab === t ? "font-bold" : "opacity-60"}`}
              style={{ color: tab === t ? "var(--accent)" : "var(--text-secondary)" }}
            >
              {tabLabels[t]}
            </button>
          ))}
        </nav>
      </header>

      <main className="flex-1 p-6">

        {/* ===== 대시보드 ===== */}
        {tab === "dashboard" && (
          <div>
            <div className="grid grid-cols-2 gap-4 mb-6">
              {[
                { label: "등록된 프로젝트", value: projects.length },
                { label: "인덱싱된 문서", value: totalDocs },
              ].map((stat) => (
                <div key={stat.label} className="p-4 rounded-lg text-center"
                  style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                  <div className="text-2xl font-bold" style={{ color: "var(--accent)" }}>{stat.value}</div>
                  <div className="text-xs opacity-60 mt-1">{stat.label}</div>
                </div>
              ))}
            </div>

            <h2 className="text-sm font-medium opacity-60 mb-3">프로젝트 현황</h2>
            <div className="space-y-2">
              {projects.map((p) => {
                const info = projectInfos.get(p.id);
                return (
                  <div key={p.id} className="p-3 rounded-lg flex items-center justify-between"
                    style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                    <div className="flex items-center gap-4">
                      <span className="font-medium">{p.name}</span>
                      {info && (
                        <span className="text-xs opacity-50">{info.stats.doc_count}개 문서</span>
                      )}
                      {info && info.stats.pending_count > 0 && (
                        <span className="text-xs px-2 py-0.5 rounded" style={{ background: "#e0af68", color: "#1a1b26" }}>
                          {info.stats.pending_count}개 대기 중
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-2">
                      <span className="text-xs opacity-40">
                        {p.last_indexed_at ? `${new Date(p.last_indexed_at).toLocaleDateString("ko-KR")} 인덱싱됨` : "미인덱싱"}
                      </span>
                      <button onClick={() => handleIndex(p.id)} disabled={indexing.has(p.id)}
                        className="px-3 py-1 rounded text-xs cursor-pointer disabled:cursor-wait"
                        style={{ background: "var(--accent)", color: "#1a1b26", opacity: indexing.has(p.id) ? 0.5 : 1 }}>
                        {indexing.has(p.id) ? "⟳ 인덱싱 중..." : "인덱싱"}
                      </button>
                    </div>
                  </div>
                );
              })}
              {projects.length === 0 && (
                <div className="text-center py-12 opacity-50">
                  <p className="text-lg mb-2">등록된 프로젝트가 없습니다</p>
                  <p className="text-sm">프로젝트 탭에서 볼트 폴더를 추가하세요</p>
                </div>
              )}
            </div>
          </div>
        )}

        {/* ===== 검색 ===== */}
        {tab === "search" && (
          <div className="flex flex-col" style={{ height: "calc(100vh - 73px)" }}>
            {/* 상단: 검색바 + 필터 + 모드 + 설정 */}
            <div className="flex-shrink-0 px-4 py-3 border-b" style={{ borderColor: "var(--border)" }}>
              {/* 검색 입력 */}
              <div className="flex gap-2 mb-2">
                <input type="text" value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && handleSearch()}
                  placeholder="문서를 검색하세요..."
                  className="flex-1 px-4 py-2.5 rounded-lg text-sm"
                  style={{ background: "var(--bg-secondary)", color: "var(--text-primary)", border: `1px solid var(--border)` }} />
                <button onClick={handleSearch} disabled={searching}
                  className="px-5 py-2.5 rounded-lg text-sm font-medium"
                  style={{ background: "var(--accent)", color: "#1a1b26", opacity: searching ? 0.5 : 1 }}>
                  {searching ? "검색 중..." : "검색"}
                </button>
                <button onClick={() => setShowSettings(!showSettings)}
                  className="px-3 py-2.5 rounded-lg text-sm"
                  style={{ background: showSettings ? "var(--accent)" : "var(--bg-secondary)", color: showSettings ? "#1a1b26" : "var(--text-secondary)", border: `1px solid var(--border)` }}>
                  ⚙
                </button>
              </div>

              {/* 필터 + 모드 */}
              <div className="flex items-center gap-3">
                {/* 프로젝트 필터 */}
                <div className="flex items-center gap-2">
                  <span className="text-xs opacity-50">프로젝트:</span>
                  <select value={selectedProject}
                    onChange={(e) => setSelectedProject(e.target.value)}
                    className="px-2 py-1 rounded text-xs"
                    style={{ background: "var(--bg-secondary)", color: "var(--text-primary)", border: `1px solid var(--border)` }}>
                    <option value="">전체</option>
                    {projects.map((p) => (
                      <option key={p.id} value={p.id}>{p.name}</option>
                    ))}
                  </select>
                </div>

                <div style={{ width: 1, height: 16, background: "var(--border)" }} />

                {/* 검색 모드 */}
                <div className="flex items-center gap-1">
                  <span className="text-xs opacity-50">모드:</span>
                  {(["hybrid", "keyword", "vector"] as SearchMode[]).map((m) => {
                    const labels: Record<SearchMode, string> = { hybrid: "하이브리드", keyword: "키워드", vector: "벡터" };
                    return (
                      <button key={m}
                        onClick={() => setSearchMode(m)}
                        className="px-2 py-0.5 rounded text-xs"
                        style={{
                          background: searchMode === m ? "var(--accent)" : "var(--bg-secondary)",
                          color: searchMode === m ? "#1a1b26" : "var(--text-secondary)",
                          border: `1px solid ${searchMode === m ? "var(--accent)" : "var(--border)"}`,
                        }}>
                        {labels[m]}
                      </button>
                    );
                  })}
                </div>

                <div style={{ width: 1, height: 16, background: "var(--border)" }} />

                {/* 태그 필터 */}
                <div className="flex items-center gap-1">
                  <span className="text-xs opacity-50">태그:</span>
                  <input type="text" value={tagFilter}
                    onChange={(e) => setTagFilter(e.target.value)}
                    onKeyDown={(e) => e.key === "Enter" && handleSearch()}
                    placeholder="rust, api"
                    className="px-2 py-0.5 rounded text-xs w-24"
                    style={{ background: "var(--bg-secondary)", color: "var(--text-primary)", border: `1px solid var(--border)` }} />
                </div>

                {results.length > 0 && (
                  <>
                    <div style={{ width: 1, height: 16, background: "var(--border)" }} />
                    <span className="text-xs opacity-50">{results.length}건 검색됨</span>
                    <button onClick={() => setResults([])} className="text-xs opacity-40 hover:opacity-100 cursor-pointer">초기화</button>
                  </>
                )}
              </div>

              {/* 세부 설정 패널 */}
              {showSettings && (
                <div className="mt-3 p-3 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-xs font-medium opacity-60">검색 세부 설정</span>
                    <button onClick={() => { setHybridWeight(0.7); setMinVectorScore(0.65); }}
                      className="px-2 py-0.5 rounded text-xs hover:opacity-80"
                      style={{ border: `1px solid var(--border)`, color: "var(--text-secondary)" }}>
                      권장값으로 되돌리기
                    </button>
                  </div>
                  <div className="grid grid-cols-2 gap-4">
                    <div>
                      <div className="flex items-center justify-between mb-1">
                        <span className="text-xs opacity-60">하이브리드 가중치 (벡터 비율)</span>
                        <span className="text-xs font-mono" style={{ color: "var(--accent)" }}>{hybridWeight.toFixed(2)}</span>
                      </div>
                      <input type="range" min="0" max="1" step="0.05" value={hybridWeight}
                        onChange={(e) => setHybridWeight(parseFloat(e.target.value))}
                        className="w-full" disabled={searchMode !== "hybrid"} />
                      <div className="flex justify-between text-xs opacity-30 mt-0.5">
                        <span>키워드 중심</span>
                        <span>벡터 중심</span>
                      </div>
                    </div>
                    <div>
                      <div className="flex items-center justify-between mb-1">
                        <span className="text-xs opacity-60">최소 벡터 유사도</span>
                        <span className="text-xs font-mono" style={{ color: "var(--accent)" }}>{minVectorScore.toFixed(2)}</span>
                      </div>
                      <input type="range" min="0" max="1" step="0.01" value={minVectorScore}
                        onChange={(e) => setMinVectorScore(parseFloat(e.target.value))}
                        className="w-full" disabled={searchMode === "keyword"} />
                      <div className="flex justify-between text-xs opacity-30 mt-0.5">
                        <span>느슨하게</span>
                        <span>엄격하게</span>
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </div>

            {/* 하단: 사이드바 + 문서 뷰어 */}
            <div className="flex flex-1 min-h-0">
              {/* 왼쪽 사이드바: 결과 + 트리 */}
              <div className="w-72 flex-shrink-0 border-r overflow-y-auto p-3" style={{ borderColor: "var(--border)" }}>
                {/* 검색 결과 — 파일별 그룹 */}
                {results.length > 0 && (() => {
                  const grouped = new Map<string, { projectId: string; projectName: string; filePath: string; items: typeof results }>();
                  for (const r of results) {
                    if (!grouped.has(r.file_path)) {
                      const project = projects.find((p) => p.name === r.project_name);
                      grouped.set(r.file_path, { projectId: project?.id || "", projectName: r.project_name, filePath: r.file_path, items: [] });
                    }
                    grouped.get(r.file_path)!.items.push(r);
                  }
                  return (
                    <div className="mb-3">
                      <div className="text-xs font-medium opacity-60 mb-2">
                        {grouped.size}개 파일에서 {results.length}건
                      </div>
                      {Array.from(grouped.values()).map((group) => {
                        const isActive = viewingDoc?.filePath === group.filePath;
                        return (
                          <div key={group.filePath} className="mb-1">
                            <div
                              className="px-2 py-1.5 rounded cursor-pointer hover:opacity-80"
                              style={{
                                background: isActive ? "var(--accent)" : "var(--bg-secondary)",
                                color: isActive ? "#1a1b26" : undefined,
                              }}
                              onClick={() => viewDocument(group.projectId, group.filePath)}>
                              <div className="flex items-center gap-1 text-xs">
                                <span className="font-medium truncate">{group.filePath.split("/").pop()}</span>
                                <span className="opacity-40 ml-auto flex-shrink-0">{group.items.length}건</span>
                              </div>
                              <div className="text-xs opacity-40 truncate">{group.projectName} / {group.filePath}</div>
                            </div>
                            {/* Collapsible match details */}
                            {group.items.length > 0 && (
                              <div className="flex items-center px-2 py-0.5">
                                <button
                                  className="text-xs opacity-40 hover:opacity-70 flex items-center gap-1"
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    setExpandedResults((prev) => {
                                      const next = new Set(prev);
                                      if (next.has(group.filePath)) next.delete(group.filePath);
                                      else next.add(group.filePath);
                                      return next;
                                    });
                                  }}>
                                  <span>{expandedResults.has(group.filePath) ? "▼" : "▶"}</span>
                                  <span>매칭 섹션</span>
                                </button>
                              </div>
                            )}
                            {expandedResults.has(group.filePath) && group.items.map((r) => (
                              <div key={r.chunk_id}
                                className="px-3 py-1 text-xs cursor-pointer hover:opacity-80"
                                style={{ color: "var(--text-secondary)" }}
                                onClick={() => viewDocument(group.projectId, group.filePath)}>
                                {r.heading_path && (
                                  <span className="opacity-60">{r.heading_path}</span>
                                )}
                                <span className="opacity-30 ml-1">({(r.score * 100).toFixed(0)}%)</span>
                              </div>
                            ))}
                          </div>
                        );
                      })}
                      <div className="border-b my-2" style={{ borderColor: "var(--border)" }} />
                    </div>
                  );
                })()}

                {/* 프로젝트 폴더 트리 */}
                <div className="text-xs font-medium opacity-60 mb-2">프로젝트</div>
                {projects.length === 0 && (
                  <p className="text-xs opacity-40 px-2">프로젝트 탭에서 볼트를 추가하세요</p>
                )}
                {projects.map((p) => {
                  const isExpanded = expandedProjects.has(p.id);
                  const docs = projectDocs.get(p.id) || [];
                  const tree = isExpanded ? buildTree(docs) : null;
                  return (
                    <div key={p.id} className="mb-1">
                      <div
                        className="flex items-center gap-1 px-2 py-1 rounded cursor-pointer hover:opacity-80"
                        style={{ background: isExpanded ? "var(--bg-secondary)" : "transparent" }}
                        onClick={() => toggleProject(p.id)}>
                        <span className="text-xs opacity-50">{isExpanded ? "▼" : "▶"}</span>
                        <span className="text-xs font-medium">{p.name}</span>
                        {projectInfos.get(p.id) && (
                          <span className="text-xs opacity-30 ml-auto">{projectInfos.get(p.id)!.stats.doc_count}</span>
                        )}
                      </div>
                      {isExpanded && tree && (
                        <div className="ml-3">
                          {Array.from(tree.folders.entries()).map(([folder, folderDocs]) => {
                            const folderKey = `${p.id}/${folder}`;
                            const isFolderOpen = expandedFolders.has(folderKey);
                            return (
                              <div key={folder} className="mb-0.5">
                                <div
                                  className="flex items-center gap-1 px-2 py-0.5 text-xs opacity-50 cursor-pointer hover:opacity-80"
                                  onClick={() => {
                                    setExpandedFolders((prev) => {
                                      const next = new Set(prev);
                                      if (next.has(folderKey)) next.delete(folderKey);
                                      else next.add(folderKey);
                                      return next;
                                    });
                                  }}>
                                  <span>{isFolderOpen ? "▼" : "▶"}</span>
                                  <span>📁 {folder}</span>
                                  <span className="ml-auto opacity-30">{folderDocs.length}</span>
                                </div>
                                {isFolderOpen && folderDocs.map((doc) => {
                                  const isActive = viewingDoc?.filePath === doc.file_path;
                                  return (
                                    <div key={doc.id}
                                      className="flex items-center gap-1 px-4 py-1 rounded cursor-pointer hover:opacity-80 text-xs"
                                      style={{
                                        background: isActive ? "var(--accent)" : "transparent",
                                        color: isActive ? "#1a1b26" : "var(--text-secondary)",
                                      }}
                                      onClick={() => viewDocument(p.id, doc.file_path)}>
                                      <span className="truncate">{doc.title || doc.file_path.split("/").pop()}</span>
                                    </div>
                                  );
                                })}
                              </div>
                            );
                          })}
                          {tree.rootDocs.map((doc) => {
                            const isActive = viewingDoc?.filePath === doc.file_path;
                            return (
                              <div key={doc.id}
                                className="flex items-center gap-1 px-2 py-1 rounded cursor-pointer hover:opacity-80 text-xs"
                                style={{
                                  background: isActive ? "var(--accent)" : "transparent",
                                  color: isActive ? "#1a1b26" : "var(--text-secondary)",
                                }}
                                onClick={() => viewDocument(p.id, doc.file_path)}>
                                <span className="truncate">{doc.title || doc.file_path}</span>
                              </div>
                            );
                          })}
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>

              {/* 오른쪽: 문서 뷰어 */}
              <div className="flex-1 overflow-y-auto">
                {viewingDoc ? (
                  <div className="p-6">
                    <div className="flex items-center justify-between mb-4">
                      <span className="text-sm opacity-60">{viewingDoc.filePath}</span>
                      <div className="flex gap-2">
                        <button onClick={() => {
                          const project = projects.find((p) => p.id === viewingDoc.projectId);
                          if (project) openFile(project, viewingDoc.filePath);
                        }}
                          className="px-3 py-1 rounded text-xs"
                          style={{ background: "var(--accent)", color: "#1a1b26" }}>
                          Obsidian에서 열기
                        </button>
                        <button onClick={() => setViewingDoc(null)}
                          className="px-3 py-1 rounded text-xs opacity-60 hover:opacity-100"
                          style={{ border: "1px solid var(--border)" }}>
                          닫기
                        </button>
                      </div>
                    </div>
                    <div className="prose prose-invert max-w-none text-sm" style={{ color: "var(--text-primary)" }}>
                      <Markdown remarkPlugins={[remarkGfm]}>{viewingDoc.content}</Markdown>
                    </div>
                  </div>
                ) : (
                  <div className="flex items-center justify-center h-full opacity-40">
                    <div className="text-center">
                      <p className="text-lg mb-2">문서를 선택하세요</p>
                      <p className="text-sm">왼쪽에서 프로젝트를 펼치거나 검색하여 문서를 확인할 수 있습니다</p>
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>
        )}

        {/* ===== 프로젝트 관리 ===== */}
        {tab === "projects" && (
          <div className="space-y-3">
            <button onClick={handleAddVault} disabled={adding}
              className="w-full px-4 py-3 rounded-lg text-sm font-medium border-2 border-dashed hover:opacity-80"
              style={{ borderColor: "var(--accent)", color: "var(--accent)", opacity: adding ? 0.5 : 1 }}>
              {adding ? "볼트 추가 및 인덱싱 중..." : "+ 볼트 폴더 추가"}
            </button>
            {projects.map((p) => {
              const info = projectInfos.get(p.id);
              return (
                <div key={p.id} className="p-4 rounded-lg"
                  style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                  <div className="flex items-center justify-between mb-2">
                    <h3 className="font-medium">{p.name}</h3>
                    <div className="flex gap-2">
                      <button onClick={() => handleIndex(p.id)} disabled={indexing.has(p.id)}
                        className="px-3 py-1 rounded text-sm cursor-pointer disabled:cursor-wait"
                        style={{ background: "var(--accent)", color: "#1a1b26", opacity: indexing.has(p.id) ? 0.5 : 1 }}>
                        {indexing.has(p.id) ? "⟳ 인덱싱 중..." : "인덱싱"}
                      </button>
                      <button onClick={() => handleSync(p.id)} disabled={syncing.has(p.id)}
                        className="px-3 py-1 rounded text-sm cursor-pointer disabled:cursor-wait"
                        style={{ border: "1px solid var(--accent)", color: "var(--accent)", opacity: syncing.has(p.id) ? 0.5 : 1 }}>
                        {syncing.has(p.id) ? "⟳ 동기화 중..." : "동기화"}
                      </button>
                      <button onClick={() => handleRemoveProject(p.id)}
                        className="px-3 py-1 rounded text-sm cursor-pointer opacity-50 hover:opacity-100"
                        style={{ border: "1px solid var(--border)", color: "var(--text-secondary)" }}>
                        삭제
                      </button>
                    </div>
                  </div>
                  <p className="text-xs opacity-50 mb-1">{p.path}</p>
                  <p className="text-xs opacity-50">
                    마지막 인덱싱: {p.last_indexed_at ? new Date(p.last_indexed_at).toLocaleString("ko-KR") : "없음"}
                  </p>
                  {info && (
                    <div className="flex gap-4 mt-2">
                      <span className="text-xs"><span style={{ color: "var(--accent)" }}>{info.stats.doc_count}</span> 문서</span>
                      {info.stats.pending_count > 0 && (
                        <span className="text-xs" style={{ color: "#e0af68" }}>{info.stats.pending_count}개 대기 중</span>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
            {projects.length === 0 && (
              <div className="text-center py-12 opacity-50">
                <p className="text-lg mb-2">등록된 프로젝트가 없습니다</p>
                <p className="text-sm">위의 "볼트 폴더 추가" 버튼을 눌러 Obsidian 볼트를 등록하세요</p>
              </div>
            )}
          </div>
        )}

        {/* ===== 가이드 ===== */}
        {tab === "settings" && <SettingsTab />}

        {tab === "guide" && (
          <div className="max-w-3xl mx-auto">
            <h2 className="text-lg font-bold mb-6" style={{ color: "var(--accent)" }}>Obsidian Nexus 사용 가이드</h2>

            {/* 데스크톱 앱 */}
            <div className="mb-8">
              <h3 className="text-base font-bold mb-3 flex items-center gap-2" style={{ color: "var(--text-primary)" }}>
                <span className="inline-block w-6 h-6 rounded text-center text-xs leading-6" style={{ background: "var(--accent)", color: "var(--bg-primary)" }}>D</span>
                데스크톱 앱
              </h3>
              <div className="space-y-3">
                <div className="p-4 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                  <h4 className="font-medium mb-2 text-sm">볼트 추가</h4>
                  <p className="text-sm opacity-70 mb-2">
                    <strong>프로젝트</strong> 탭에서 "볼트 폴더 추가" 버튼을 클릭하세요.
                    Obsidian 볼트 폴더를 선택하면 자동으로 등록, 인덱싱, Obsidian 연동이 완료됩니다.
                  </p>
                  <div className="text-xs p-2 rounded" style={{ background: "var(--bg-primary)", borderLeft: `3px solid var(--accent)` }}>
                    <strong>사전 준비:</strong> 등록하려는 폴더를 먼저 Obsidian 앱에서 한 번 열어 볼트로 초기화해야 합니다.
                    Obsidian이 <code className="px-1 rounded" style={{ background: "var(--bg-secondary)" }}>.obsidian/</code> 폴더를 생성해야 Nexus가 볼트로 인식할 수 있습니다.
                  </div>
                </div>
                <div className="p-4 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                  <h4 className="font-medium mb-2 text-sm">문서 검색</h4>
                  <p className="text-sm opacity-70">
                    <strong>검색</strong> 탭에서 키워드를 입력하면 모든 볼트에서 문서를 찾습니다.
                    키워드 / 벡터 / 하이브리드 검색 모드를 지원하며, 프로젝트와 태그 필터링이 가능합니다.
                  </p>
                </div>
                <div className="p-4 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                  <h4 className="font-medium mb-2 text-sm">문서 탐색 & 편집</h4>
                  <p className="text-sm opacity-70">
                    검색 결과를 클릭하면 마크다운으로 렌더링된 내용을 확인할 수 있고,
                    "Obsidian에서 열기" 버튼으로 Obsidian 앱에서 바로 편집할 수 있습니다.
                  </p>
                </div>
                <div className="p-4 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                  <h4 className="font-medium mb-2 text-sm">자동 설정</h4>
                  <p className="text-sm opacity-70">
                    앱 실행 시 자동으로 MCP 서버를 AI 도구(Claude Desktop, Claude Code, Gemini CLI)에 등록하고,
                    터미널에서 <code className="px-1 rounded text-xs" style={{ background: "var(--bg-primary)" }}>nexus</code> 명령어를 바로 사용할 수 있도록 PATH에 추가합니다.
                  </p>
                </div>
              </div>
            </div>

            {/* CLI */}
            <div className="mb-8">
              <h3 className="text-base font-bold mb-3 flex items-center gap-2" style={{ color: "var(--text-primary)" }}>
                <span className="inline-block w-6 h-6 rounded text-center text-xs leading-6" style={{ background: "var(--accent)", color: "var(--bg-primary)" }}>C</span>
                CLI (Command Line Interface)
              </h3>
              <p className="text-sm opacity-70 mb-3">
                데스크톱 앱 설치 시 자동으로 포함됩니다. 터미널에서 <code className="px-1 rounded text-xs" style={{ background: "var(--bg-primary)" }}>nexus</code> 명령어로 사용하세요.
              </p>
              <div className="space-y-3">
                <div className="p-4 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                  <h4 className="font-medium mb-2 text-sm">프로젝트 관리</h4>
                  <div className="text-xs p-3 rounded space-y-1 font-mono" style={{ background: "var(--bg-primary)" }}>
                    <p><span className="opacity-50"># 볼트 등록</span></p>
                    <p>nexus project add "My Vault" /path/to/vault</p>
                    <p><span className="opacity-50"># 프로젝트 목록</span></p>
                    <p>nexus project list</p>
                    <p><span className="opacity-50"># 프로젝트 상세 정보</span></p>
                    <p>nexus project info "project-id"</p>
                  </div>
                </div>
                <div className="p-4 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                  <h4 className="font-medium mb-2 text-sm">검색</h4>
                  <div className="text-xs p-3 rounded space-y-1 font-mono" style={{ background: "var(--bg-primary)" }}>
                    <p><span className="opacity-50"># 키워드 검색 (기본)</span></p>
                    <p>nexus search "검색어"</p>
                    <p><span className="opacity-50"># 특정 프로젝트에서 벡터 검색</span></p>
                    <p>nexus search "의미 검색" --project "My Vault" --mode vector</p>
                    <p><span className="opacity-50"># 하이브리드 검색 (키워드 + 벡터)</span></p>
                    <p>nexus search "질문" --mode hybrid --limit 10</p>
                  </div>
                </div>
                <div className="p-4 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                  <h4 className="font-medium mb-2 text-sm">문서 접근</h4>
                  <div className="text-xs p-3 rounded space-y-1 font-mono" style={{ background: "var(--bg-primary)" }}>
                    <p><span className="opacity-50"># 문서 내용 가져오기</span></p>
                    <p>nexus doc get "project-id" "notes/file.md"</p>
                    <p><span className="opacity-50"># 문서 메타데이터 (태그, 링크)</span></p>
                    <p>nexus doc meta "project-id" "notes/file.md"</p>
                    <p><span className="opacity-50"># 프로젝트 문서 목록</span></p>
                    <p>nexus doc list "project-id"</p>
                  </div>
                </div>
                <div className="p-4 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                  <h4 className="font-medium mb-2 text-sm">인덱싱 & 자동 감시</h4>
                  <div className="text-xs p-3 rounded space-y-1 font-mono" style={{ background: "var(--bg-primary)" }}>
                    <p><span className="opacity-50"># 수동 인덱싱 (변경분만)</span></p>
                    <p>nexus index "project-id"</p>
                    <p><span className="opacity-50"># 실시간 파일 감시 + 자동 인덱싱</span></p>
                    <p>nexus watch "project-id"</p>
                  </div>
                </div>
                <div className="p-4 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                  <h4 className="font-medium mb-2 text-sm">프로젝트 온보딩</h4>
                  <div className="text-xs p-3 rounded space-y-1 font-mono" style={{ background: "var(--bg-primary)" }}>
                    <p><span className="opacity-50"># 현재 프로젝트에 MCP + librarian 스킬 자동 설정</span></p>
                    <p>nexus onboard</p>
                    <p><span className="opacity-50"># 특정 경로 지정</span></p>
                    <p>nexus onboard /path/to/project</p>
                  </div>
                  <p className="text-xs opacity-60 mt-2">.mcp.json, .claude/agents, .claude/skills 파일이 자동 생성됩니다.</p>
                </div>
              </div>
            </div>

            {/* MCP 서버 */}
            <div className="mb-8">
              <h3 className="text-base font-bold mb-3 flex items-center gap-2" style={{ color: "var(--text-primary)" }}>
                <span className="inline-block w-6 h-6 rounded text-center text-xs leading-6" style={{ background: "var(--accent)", color: "var(--bg-primary)" }}>M</span>
                MCP 서버 (AI 에이전트 연동)
              </h3>
              <p className="text-sm opacity-70 mb-3">
                앱 실행 시 자동으로 AI 도구에 등록됩니다. Claude Code, Claude Desktop, Gemini CLI에서 자동 사용 가능합니다.
              </p>
              <div className="space-y-3">
                <div className="p-4 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                  <h4 className="font-medium mb-2 text-sm">검색 도구</h4>
                  <div className="text-xs space-y-2">
                    <div className="flex gap-2">
                      <code className="px-1.5 py-0.5 rounded shrink-0" style={{ background: "var(--bg-primary)" }}>nexus_search</code>
                      <span className="opacity-70">하이브리드/키워드/벡터 검색, 태그 필터, 인기도 부스트</span>
                    </div>
                    <div className="flex gap-2">
                      <code className="px-1.5 py-0.5 rounded shrink-0" style={{ background: "var(--bg-primary)" }}>nexus_resolve_alias</code>
                      <span className="opacity-70">별칭으로 문서 찾기</span>
                    </div>
                  </div>
                </div>
                <div className="p-4 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                  <h4 className="font-medium mb-2 text-sm">문서 접근 도구</h4>
                  <div className="text-xs space-y-2">
                    <div className="flex gap-2">
                      <code className="px-1.5 py-0.5 rounded shrink-0" style={{ background: "var(--bg-primary)" }}>nexus_get_document</code>
                      <span className="opacity-70">문서 전체 내용 가져오기</span>
                    </div>
                    <div className="flex gap-2">
                      <code className="px-1.5 py-0.5 rounded shrink-0" style={{ background: "var(--bg-primary)" }}>nexus_get_section</code>
                      <span className="opacity-70">특정 섹션만 가져오기 (토큰 절약!)</span>
                    </div>
                    <div className="flex gap-2">
                      <code className="px-1.5 py-0.5 rounded shrink-0" style={{ background: "var(--bg-primary)" }}>nexus_get_metadata</code>
                      <span className="opacity-70">프론트매터, 태그, 인덱싱 상태</span>
                    </div>
                  </div>
                </div>
                <div className="p-4 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                  <h4 className="font-medium mb-2 text-sm">그래프 탐색 도구</h4>
                  <div className="text-xs space-y-2">
                    <div className="flex gap-2">
                      <code className="px-1.5 py-0.5 rounded shrink-0" style={{ background: "var(--bg-primary)" }}>nexus_get_backlinks</code>
                      <span className="opacity-70">이 문서를 링크하는 문서들</span>
                    </div>
                    <div className="flex gap-2">
                      <code className="px-1.5 py-0.5 rounded shrink-0" style={{ background: "var(--bg-primary)" }}>nexus_get_links</code>
                      <span className="opacity-70">이 문서가 링크하는 문서들</span>
                    </div>
                  </div>
                </div>
                <div className="p-4 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                  <h4 className="font-medium mb-2 text-sm">관리 도구</h4>
                  <div className="text-xs space-y-2">
                    <div className="flex gap-2">
                      <code className="px-1.5 py-0.5 rounded shrink-0" style={{ background: "var(--bg-primary)" }}>nexus_list_projects</code>
                      <span className="opacity-70">등록된 볼트 목록</span>
                    </div>
                    <div className="flex gap-2">
                      <code className="px-1.5 py-0.5 rounded shrink-0" style={{ background: "var(--bg-primary)" }}>nexus_list_documents</code>
                      <span className="opacity-70">프로젝트 문서 목록 (태그 필터)</span>
                    </div>
                    <div className="flex gap-2">
                      <code className="px-1.5 py-0.5 rounded shrink-0" style={{ background: "var(--bg-primary)" }}>nexus_index_project</code>
                      <span className="opacity-70">증분 / 전체 리인덱스</span>
                    </div>
                    <div className="flex gap-2">
                      <code className="px-1.5 py-0.5 rounded shrink-0" style={{ background: "var(--bg-primary)" }}>nexus_status</code>
                      <span className="opacity-70">시스템 상태 확인 (Ollama, DB)</span>
                    </div>
                    <div className="flex gap-2">
                      <code className="px-1.5 py-0.5 rounded shrink-0" style={{ background: "var(--bg-primary)" }}>nexus_onboard</code>
                      <span className="opacity-70">프로젝트에 librarian 스킬 자동 설정</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* 추천 워크플로우 */}
            <div className="p-4 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--accent)33` }}>
              <h3 className="font-medium mb-2 text-sm" style={{ color: "var(--accent)" }}>추천 워크플로우</h3>
              <div className="text-sm opacity-70 space-y-1">
                <p>1. 데스크톱 앱으로 Obsidian 볼트 등록 & 인덱싱</p>
                <p>2. <code className="px-1 rounded text-xs" style={{ background: "var(--bg-primary)" }}>nexus onboard</code>로 프로젝트에 MCP 연동 설정</p>
                <p>3. AI 에이전트가 <code className="px-1 rounded text-xs" style={{ background: "var(--bg-primary)" }}>nexus_search</code> → <code className="px-1 rounded text-xs" style={{ background: "var(--bg-primary)" }}>nexus_get_section</code>으로 필요한 문서만 가져옴</p>
                <p>4. <code className="px-1 rounded text-xs" style={{ background: "var(--bg-primary)" }}>nexus_get_backlinks</code>로 관련 문서 탐색</p>
              </div>
            </div>
          </div>
        )}

      </main>
    </div>
  );
}

export default App;
