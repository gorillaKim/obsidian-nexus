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

type Tab = "dashboard" | "search" | "projects" | "guide";

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
  const [viewingDoc, setViewingDoc] = useState<{ projectId: string; filePath: string; content: string } | null>(null);
  const [browseDocs, setBrowseDocs] = useState<{ projectId: string; projectName: string; docs: DocItem[] } | null>(null);

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
    setBrowseDocs(null);
    try {
      const res = await invoke<SearchResult[]>("search_documents", {
        query, projectId: selectedProject || null, limit: 20,
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
      const folderName = folderPath.split("/").pop() || "untitled";
      await invoke("add_project", { name: folderName, path: folderPath });
      await loadProjects();
    } catch (e) { console.error(e); }
    setAdding(false);
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

  const loadProjectDocs = async (projectId: string, projectName: string) => {
    try {
      const docs = await invoke<DocItem[]>("list_documents", { projectId });
      setBrowseDocs({ projectId, projectName, docs });
      setResults([]);
      setViewingDoc(null);
    } catch (e) { console.error(e); }
  };

  const totalDocs = Array.from(projectInfos.values()).reduce((sum, i) => sum + i.stats.doc_count, 0);

  const tabLabels: Record<Tab, string> = {
    dashboard: "대시보드",
    search: "검색",
    projects: "프로젝트",
    guide: "가이드",
  };

  return (
    <div className="min-h-screen flex flex-col" style={{ background: "var(--bg-primary)" }}>
      {/* 헤더 */}
      <header className="px-6 py-4 flex items-center gap-4 border-b sticky top-0 z-50" style={{ borderColor: "var(--border)", background: "var(--bg-primary)" }}>
        <h1 className="text-xl font-bold" style={{ color: "var(--accent)" }}>Obsidian Nexus</h1>
        <nav className="flex gap-2 ml-auto">
          {(["dashboard", "search", "projects", "guide"] as Tab[]).map((t) => (
            <button
              key={t}
              onClick={() => { setTab(t); setViewingDoc(null); setBrowseDocs(null); }}
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
                        className="px-3 py-1 rounded text-xs"
                        style={{ background: "var(--accent)", color: "#1a1b26", opacity: indexing.has(p.id) ? 0.5 : 1 }}>
                        {indexing.has(p.id) ? "인덱싱 중..." : "인덱싱"}
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
          <div>
            {/* 검색바 */}
            <div className="flex gap-2 mb-4">
              <select value={selectedProject}
                onChange={(e) => { setSelectedProject(e.target.value); setBrowseDocs(null); setViewingDoc(null); }}
                className="px-3 py-2 rounded text-sm"
                style={{ background: "var(--bg-secondary)", color: "var(--text-primary)", border: `1px solid var(--border)` }}>
                <option value="">전체 프로젝트</option>
                {projects.map((p) => (
                  <option key={p.id} value={p.id}>{p.name}</option>
                ))}
              </select>
              <input type="text" value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleSearch()}
                placeholder="지식 베이스 검색..."
                className="flex-1 px-4 py-2 rounded text-sm"
                style={{ background: "var(--bg-secondary)", color: "var(--text-primary)", border: `1px solid var(--border)` }} />
              <button onClick={handleSearch} disabled={searching}
                className="px-4 py-2 rounded text-sm font-medium"
                style={{ background: "var(--accent)", color: "#1a1b26" }}>
                {searching ? "..." : "검색"}
              </button>
            </div>

            {/* 프로젝트별 문서 탐색 버튼 */}
            {!browseDocs && results.length === 0 && !viewingDoc && (
              <div className="mb-4">
                <h3 className="text-sm opacity-60 mb-2">프로젝트별 문서 탐색</h3>
                <div className="flex gap-2 flex-wrap">
                  {projects.map((p) => {
                    const info = projectInfos.get(p.id);
                    return (
                      <button key={p.id}
                        onClick={() => loadProjectDocs(p.id, p.name)}
                        className="px-3 py-2 rounded-lg text-sm hover:opacity-80"
                        style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                        {p.name}
                        {info && <span className="text-xs opacity-50 ml-2">{info.stats.doc_count}개</span>}
                      </button>
                    );
                  })}
                </div>
                {projects.length === 0 && (
                  <p className="text-center opacity-40 py-8">프로젝트 탭에서 볼트를 먼저 추가하세요</p>
                )}
              </div>
            )}

            {/* 메인 영역: 목록 + 뷰어 */}
            <div className="flex gap-4" style={{ height: "calc(100vh - 200px)" }}>

              {/* 왼쪽: 검색 결과 또는 문서 목록 */}
              <div className={`space-y-2 overflow-y-auto ${viewingDoc ? "w-1/3" : "w-full"}`}>

                {/* 문서 탐색 모드 */}
                {browseDocs && !results.length && (
                  <>
                    <div className="flex items-center justify-between mb-2">
                      <span className="text-sm font-medium">{browseDocs.projectName} — {browseDocs.docs.length}개 문서</span>
                      <button onClick={() => { setBrowseDocs(null); setViewingDoc(null); }}
                        className="text-xs opacity-50 hover:opacity-100">닫기</button>
                    </div>
                    {browseDocs.docs.map((doc) => {
                      const isActive = viewingDoc?.filePath === doc.file_path;
                      return (
                        <div key={doc.id}
                          className="p-3 rounded-lg cursor-pointer hover:opacity-90 flex items-center gap-3"
                          style={{
                            background: isActive ? "var(--accent)" : "var(--bg-secondary)",
                            color: isActive ? "#1a1b26" : undefined,
                            border: `1px solid ${isActive ? "var(--accent)" : "var(--border)"}`,
                          }}
                          onClick={() => viewDocument(browseDocs.projectId, doc.file_path)}>
                          <div>
                            <div className="text-sm font-medium">{doc.title || doc.file_path}</div>
                            {doc.title && <div className="text-xs opacity-40">{doc.file_path}</div>}
                          </div>
                        </div>
                      );
                    })}
                  </>
                )}

                {/* 검색 결과 모드 */}
                {results.length > 0 && (
                  <>
                    {browseDocs && (
                      <button onClick={() => { setResults([]); }}
                        className="text-xs opacity-50 hover:opacity-100 mb-2">문서 목록으로 돌아가기</button>
                    )}
                    {results.map((r) => {
                      const project = projects.find((p) => p.name === r.project_name);
                      const isActive = viewingDoc?.filePath === r.file_path;
                      return (
                        <div key={r.chunk_id}
                          className="p-3 rounded-lg cursor-pointer hover:opacity-90"
                          style={{
                            background: isActive ? "var(--accent)" : "var(--bg-secondary)",
                            color: isActive ? "#1a1b26" : undefined,
                            border: `1px solid ${isActive ? "var(--accent)" : "var(--border)"}`,
                          }}
                          onClick={() => { if (project) viewDocument(project.id, r.file_path); }}>
                          <div className="flex items-center gap-2 mb-1">
                            {!isActive && (
                              <span className="text-xs px-2 py-0.5 rounded" style={{ background: "var(--accent)", color: "#1a1b26" }}>
                                {r.project_name}
                              </span>
                            )}
                            <span className="text-sm opacity-70">{r.file_path}</span>
                          </div>
                          {r.heading_path && <div className="text-xs opacity-50 mb-1">{r.heading_path}</div>}
                          {!viewingDoc && (
                            <p className="text-sm" style={{ color: isActive ? "#1a1b26" : "var(--text-secondary)" }}
                              dangerouslySetInnerHTML={{ __html: r.snippet }} />
                          )}
                        </div>
                      );
                    })}
                  </>
                )}

                {results.length === 0 && query && !searching && !browseDocs && (
                  <p className="text-center opacity-50 py-8">검색 결과가 없습니다</p>
                )}
              </div>

              {/* 오른쪽: 문서 뷰어 */}
              {viewingDoc && (
                <div className="w-2/3 overflow-y-auto rounded-lg p-6"
                  style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
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
              )}
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
                        className="px-3 py-1 rounded text-sm"
                        style={{ background: "var(--accent)", color: "#1a1b26", opacity: indexing.has(p.id) ? 0.5 : 1 }}>
                        {indexing.has(p.id) ? "인덱싱 중..." : "인덱싱"}
                      </button>
                      <button onClick={() => handleRemoveProject(p.id)}
                        className="px-3 py-1 rounded text-sm opacity-50 hover:opacity-100"
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
        {tab === "guide" && (
          <div className="max-w-2xl mx-auto">
            <h2 className="text-lg font-bold mb-4" style={{ color: "var(--accent)" }}>Obsidian Nexus 사용 가이드</h2>

            <div className="space-y-4">
              <div className="p-4 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                <h3 className="font-medium mb-2">1. 볼트 추가</h3>
                <p className="text-sm opacity-70">
                  <strong>프로젝트</strong> 탭에서 "볼트 폴더 추가" 버튼을 클릭하세요.
                  Obsidian 볼트 폴더를 선택하면 자동으로 등록, 인덱싱, Obsidian 연동이 완료됩니다.
                </p>
              </div>

              <div className="p-4 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                <h3 className="font-medium mb-2">2. 문서 검색</h3>
                <p className="text-sm opacity-70">
                  <strong>검색</strong> 탭에서 키워드를 입력하면 모든 볼트에서 문서를 찾습니다.
                  특정 프로젝트만 검색하려면 드롭다운에서 선택하세요.
                  검색 결과를 클릭하면 문서 내용을 바로 확인할 수 있습니다.
                </p>
              </div>

              <div className="p-4 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                <h3 className="font-medium mb-2">3. 문서 탐색</h3>
                <p className="text-sm opacity-70">
                  <strong>검색</strong> 탭 하단의 프로젝트 버튼을 클릭하면 해당 프로젝트의 모든 문서 목록을 볼 수 있습니다.
                  문서를 클릭하면 마크다운으로 렌더링된 내용을 확인할 수 있고,
                  "Obsidian에서 열기" 버튼으로 Obsidian 앱에서 편집할 수 있습니다.
                </p>
              </div>

              <div className="p-4 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                <h3 className="font-medium mb-2">4. 에이전트 연동 (CLI / MCP)</h3>
                <p className="text-sm opacity-70 mb-2">
                  AI 에이전트가 CLI 또는 MCP를 통해 문서를 검색하고 내용을 가져올 수 있습니다.
                </p>
                <div className="text-xs p-3 rounded" style={{ background: "var(--bg-primary)" }}>
                  <p className="opacity-60 mb-1"># CLI 검색</p>
                  <p>nexus search "검색어" --project "프로젝트명"</p>
                  <p className="opacity-60 mt-2 mb-1"># MCP (Claude Code에서 자동 사용)</p>
                  <p>앱 설치 시 자동으로 MCP 서버가 등록됩니다</p>
                </div>
              </div>

              <div className="p-4 rounded-lg" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                <h3 className="font-medium mb-2">5. 인덱싱</h3>
                <p className="text-sm opacity-70">
                  볼트에 문서를 추가/수정하면 <strong>프로젝트</strong> 탭에서 "인덱싱" 버튼을 눌러 업데이트하세요.
                  변경된 문서만 자동으로 감지하여 빠르게 인덱싱됩니다.
                  CLI에서 <code className="px-1 rounded text-xs" style={{ background: "var(--bg-primary)" }}>nexus watch</code>를 실행하면 실시간 자동 인덱싱도 가능합니다.
                </p>
              </div>
            </div>
          </div>
        )}

      </main>
    </div>
  );
}

export default App;
