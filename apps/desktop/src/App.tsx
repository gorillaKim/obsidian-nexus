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

type Tab = "dashboard" | "search" | "projects";

function App() {
  const [tab, setTab] = useState<Tab>("dashboard");
  const [projects, setProjects] = useState<Project[]>([]);
  const [projectInfos, setProjectInfos] = useState<Map<string, ProjectInfo>>(new Map());
  const [query, setQuery] = useState("");
  const [selectedProject, setSelectedProject] = useState<string>("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [indexing, setIndexing] = useState<Set<string>>(new Set());
  const [viewingDoc, setViewingDoc] = useState<{ projectId: string; filePath: string; content: string } | null>(null);

  const loadProjects = useCallback(async () => {
    try {
      const list = await invoke<Project[]>("list_projects");
      setProjects(list);

      // Load stats for each project
      const infos = new Map<string, ProjectInfo>();
      for (const p of list) {
        try {
          const info = await invoke<ProjectInfo>("project_info", { projectId: p.id });
          infos.set(p.id, info);
        } catch {
          // skip
        }
      }
      setProjectInfos(infos);
    } catch (e) {
      console.error("Failed to load projects:", e);
    }
  }, []);

  useEffect(() => {
    loadProjects();
  }, [loadProjects]);

  const handleSearch = async () => {
    if (!query.trim()) return;
    setSearching(true);
    try {
      const res = await invoke<SearchResult[]>("search_documents", {
        query,
        projectId: selectedProject || null,
        limit: 20,
      });
      setResults(res);
    } catch (e) {
      console.error("Search failed:", e);
    }
    setSearching(false);
  };

  const handleIndex = async (projectId: string) => {
    setIndexing((prev) => new Set(prev).add(projectId));
    try {
      await invoke("index_project", { projectId });
      await loadProjects();
    } catch (e) {
      console.error("Indexing failed:", e);
    }
    setIndexing((prev) => {
      const next = new Set(prev);
      next.delete(projectId);
      return next;
    });
  };

  const [adding, setAdding] = useState(false);

  const handleAddVault = async () => {
    try {
      const selected = await open({ directory: true, title: "Select Obsidian Vault Folder" });
      if (!selected) return;
      setAdding(true);
      const folderPath = selected as string;
      const folderName = folderPath.split("/").pop() || "untitled";
      await invoke("add_project", { name: folderName, path: folderPath });
      await loadProjects();
    } catch (e) {
      console.error("Failed to add vault:", e);
    }
    setAdding(false);
  };

  const handleRemoveProject = async (projectId: string) => {
    try {
      await invoke("remove_project", { projectId });
      await loadProjects();
    } catch (e) {
      console.error("Failed to remove project:", e);
    }
  };

  const openFile = async (project: Project, filePath: string) => {
    try {
      await invoke("open_file", { projectId: project.id, filePath });
    } catch (e) {
      console.error("Failed to open file:", e);
    }
  };

  const viewDocument = async (projectId: string, filePath: string) => {
    try {
      const content = await invoke<string>("get_document", { projectId, filePath });
      setViewingDoc({ projectId, filePath, content });
    } catch (e) {
      console.error("Failed to load document:", e);
    }
  };

  // Totals
  const totalDocs = Array.from(projectInfos.values()).reduce((sum, i) => sum + i.stats.doc_count, 0);
  const totalChunks = Array.from(projectInfos.values()).reduce((sum, i) => sum + i.stats.chunk_count, 0);
  return (
    <div className="min-h-screen flex flex-col" style={{ background: "var(--bg-primary)" }}>
      {/* Header */}
      <header className="px-6 py-4 flex items-center gap-4 border-b" style={{ borderColor: "var(--border)" }}>
        <h1 className="text-xl font-bold" style={{ color: "var(--accent)" }}>
          Obsidian Nexus
        </h1>
        <nav className="flex gap-2 ml-auto">
          {(["dashboard", "search", "projects"] as Tab[]).map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              className={`px-3 py-1 rounded text-sm ${tab === t ? "font-bold" : "opacity-60"}`}
              style={{ color: tab === t ? "var(--accent)" : "var(--text-secondary)" }}
            >
              {t.charAt(0).toUpperCase() + t.slice(1)}
            </button>
          ))}
        </nav>
      </header>

      {/* Content */}
      <main className="flex-1 p-6">
        {/* Dashboard */}
        {tab === "dashboard" && (
          <div>
            {/* Stats Cards */}
            <div className="grid grid-cols-3 gap-4 mb-6">
              {[
                { label: "Projects", value: projects.length },
                { label: "Documents", value: totalDocs },
                { label: "Chunks", value: totalChunks },
              ].map((stat) => (
                <div
                  key={stat.label}
                  className="p-4 rounded-lg text-center"
                  style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}
                >
                  <div className="text-2xl font-bold" style={{ color: "var(--accent)" }}>
                    {stat.value}
                  </div>
                  <div className="text-xs opacity-60 mt-1">{stat.label}</div>
                </div>
              ))}
            </div>

            {/* Project Summary */}
            <h2 className="text-sm font-medium opacity-60 mb-3">Project Overview</h2>
            <div className="space-y-2">
              {projects.map((p) => {
                const info = projectInfos.get(p.id);
                return (
                  <div
                    key={p.id}
                    className="p-3 rounded-lg flex items-center justify-between"
                    style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}
                  >
                    <div className="flex items-center gap-4">
                      <span className="font-medium">{p.name}</span>
                      {info && (
                        <span className="text-xs opacity-50">
                          {info.stats.doc_count} docs / {info.stats.chunk_count} chunks
                        </span>
                      )}
                      {info && info.stats.pending_count > 0 && (
                        <span className="text-xs px-2 py-0.5 rounded" style={{ background: "#e0af68", color: "#1a1b26" }}>
                          {info.stats.pending_count} pending
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-2">
                      <span className="text-xs opacity-40">
                        {p.last_indexed_at ? `Indexed: ${new Date(p.last_indexed_at).toLocaleDateString()}` : "Not indexed"}
                      </span>
                      <button
                        onClick={() => handleIndex(p.id)}
                        disabled={indexing.has(p.id)}
                        className="px-3 py-1 rounded text-xs"
                        style={{ background: "var(--accent)", color: "#1a1b26", opacity: indexing.has(p.id) ? 0.5 : 1 }}
                      >
                        {indexing.has(p.id) ? "Indexing..." : "Index"}
                      </button>
                    </div>
                  </div>
                );
              })}
              {projects.length === 0 && (
                <div className="text-center py-12 opacity-50">
                  <p className="text-lg mb-2">No projects registered</p>
                  <p className="text-sm">
                    Use CLI: <code className="px-1 rounded" style={{ background: "var(--bg-secondary)" }}>nexus project add --name "my-vault" --path /path</code>
                  </p>
                </div>
              )}
            </div>
          </div>
        )}

        {/* Search */}
        {tab === "search" && (
          <div>
            <div className="flex gap-2 mb-4">
              <select
                value={selectedProject}
                onChange={(e) => setSelectedProject(e.target.value)}
                className="px-3 py-2 rounded text-sm"
                style={{ background: "var(--bg-secondary)", color: "var(--text-primary)", border: `1px solid var(--border)` }}
              >
                <option value="">All Projects</option>
                {projects.map((p) => (
                  <option key={p.id} value={p.id}>{p.name}</option>
                ))}
              </select>
              <input
                type="text"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleSearch()}
                placeholder="Search your knowledge base..."
                className="flex-1 px-4 py-2 rounded text-sm"
                style={{ background: "var(--bg-secondary)", color: "var(--text-primary)", border: `1px solid var(--border)` }}
              />
              <button
                onClick={handleSearch}
                disabled={searching}
                className="px-4 py-2 rounded text-sm font-medium"
                style={{ background: "var(--accent)", color: "#1a1b26" }}
              >
                {searching ? "..." : "Search"}
              </button>
            </div>

            <div className="flex gap-4" style={{ height: "calc(100vh - 160px)" }}>
              {/* Results list */}
              <div className={`space-y-2 overflow-y-auto ${viewingDoc ? "w-1/3" : "w-full"}`}>
                {results.map((r) => {
                  const project = projects.find((p) => p.name === r.project_name);
                  const isActive = viewingDoc?.filePath === r.file_path;
                  return (
                    <div
                      key={r.chunk_id}
                      className="p-3 rounded-lg cursor-pointer hover:opacity-90"
                      style={{
                        background: isActive ? "var(--accent)" : "var(--bg-secondary)",
                        color: isActive ? "#1a1b26" : undefined,
                        border: `1px solid ${isActive ? "var(--accent)" : "var(--border)"}`,
                      }}
                      onClick={() => {
                        if (project) viewDocument(project.id, r.file_path);
                      }}
                    >
                      <div className="flex items-center gap-2 mb-1">
                        {!isActive && (
                          <span className="text-xs px-2 py-0.5 rounded" style={{ background: "var(--accent)", color: "#1a1b26" }}>
                            {r.project_name}
                          </span>
                        )}
                        <span className="text-sm opacity-70">{r.file_path}</span>
                      </div>
                      {r.heading_path && (
                        <div className="text-xs opacity-50 mb-1">{r.heading_path}</div>
                      )}
                      {!viewingDoc && (
                        <p
                          className="text-sm"
                          style={{ color: isActive ? "#1a1b26" : "var(--text-secondary)" }}
                          dangerouslySetInnerHTML={{ __html: r.snippet }}
                        />
                      )}
                    </div>
                  );
                })}
                {results.length === 0 && query && !searching && (
                  <p className="text-center opacity-50 py-8">No results found</p>
                )}
                {!query && !viewingDoc && (
                  <p className="text-center opacity-40 py-12">Enter a search query to find documents across your vaults</p>
                )}
              </div>

              {/* Document viewer */}
              {viewingDoc && (
                <div className="w-2/3 overflow-y-auto rounded-lg p-6" style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}>
                  <div className="flex items-center justify-between mb-4">
                    <span className="text-sm opacity-60">{viewingDoc.filePath}</span>
                    <div className="flex gap-2">
                      <button
                        onClick={() => {
                          const project = projects.find((p) => p.id === viewingDoc.projectId);
                          if (project) openFile(project, viewingDoc.filePath);
                        }}
                        className="px-3 py-1 rounded text-xs"
                        style={{ background: "var(--accent)", color: "#1a1b26" }}
                      >
                        Open in Obsidian
                      </button>
                      <button
                        onClick={() => setViewingDoc(null)}
                        className="px-3 py-1 rounded text-xs opacity-60 hover:opacity-100"
                        style={{ border: "1px solid var(--border)" }}
                      >
                        Close
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

        {/* Projects */}
        {tab === "projects" && (
          <div className="space-y-3">
            <button
              onClick={handleAddVault}
              disabled={adding}
              className="w-full px-4 py-3 rounded-lg text-sm font-medium border-2 border-dashed hover:opacity-80"
              style={{ borderColor: "var(--accent)", color: "var(--accent)", opacity: adding ? 0.5 : 1 }}
            >
              {adding ? "Adding vault & indexing..." : "+ Add Vault Folder"}
            </button>
            {projects.map((p) => {
              const info = projectInfos.get(p.id);
              return (
                <div
                  key={p.id}
                  className="p-4 rounded-lg"
                  style={{ background: "var(--bg-secondary)", border: `1px solid var(--border)` }}
                >
                  <div className="flex items-center justify-between mb-2">
                    <h3 className="font-medium">{p.name}</h3>
                    <div className="flex gap-2">
                      <button
                        onClick={() => handleIndex(p.id)}
                        disabled={indexing.has(p.id)}
                        className="px-3 py-1 rounded text-sm"
                        style={{ background: "var(--accent)", color: "#1a1b26", opacity: indexing.has(p.id) ? 0.5 : 1 }}
                      >
                        {indexing.has(p.id) ? "Indexing..." : "Index Now"}
                      </button>
                      <button
                        onClick={() => handleRemoveProject(p.id)}
                        className="px-3 py-1 rounded text-sm opacity-50 hover:opacity-100"
                        style={{ border: "1px solid var(--border)", color: "var(--text-secondary)" }}
                      >
                        Remove
                      </button>
                    </div>
                  </div>
                  <p className="text-xs opacity-50 mb-1">{p.path}</p>
                  {p.vault_name && (
                    <p className="text-xs opacity-50 mb-1">Vault: {p.vault_name}</p>
                  )}
                  <p className="text-xs opacity-50">
                    Last indexed: {p.last_indexed_at ? new Date(p.last_indexed_at).toLocaleString() : "Never"}
                  </p>
                  {info && (
                    <div className="flex gap-4 mt-2">
                      {[
                        { label: "Docs", value: info.stats.doc_count },
                        { label: "Chunks", value: info.stats.chunk_count },
                      ].map((s) => (
                        <span key={s.label} className="text-xs">
                          <span style={{ color: "var(--accent)" }}>{s.value}</span>{" "}
                          <span className="opacity-50">{s.label}</span>
                        </span>
                      ))}
                      {info.stats.pending_count > 0 && (
                        <span className="text-xs" style={{ color: "#e0af68" }}>
                          {info.stats.pending_count} pending
                        </span>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
            {projects.length === 0 && (
              <div className="text-center py-12 opacity-50">
                <p className="text-lg mb-2">No projects registered</p>
                <p className="text-sm">
                  Use CLI: <code className="px-1 rounded" style={{ background: "var(--bg-secondary)" }}>nexus project add --name "my-vault" --path /path</code>
                </p>
              </div>
            )}
          </div>
        )}
      </main>
    </div>
  );
}

export default App;
