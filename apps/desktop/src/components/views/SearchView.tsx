import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  Search, Settings2, ChevronDown, ChevronRight,
  FolderOpen, FileText, ExternalLink, X, RefreshCw,
} from "lucide-react";
import { cn } from "../../lib/utils";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { IconButton } from "../ui/IconButton";
import { Card } from "../ui/Card";
import type { Project, ProjectInfo, SearchResult, SearchMode, DocItem } from "../../types";
import type { TreeData, TreeNode } from "../../hooks/useProjectTree";

interface SearchViewProps {
  // Search state
  query: string;
  setQuery: (q: string) => void;
  selectedProject: string;
  setSelectedProject: (p: string) => void;
  results: SearchResult[];
  clearResults: () => void;
  searching: boolean;
  searchMode: SearchMode;
  setSearchMode: (m: SearchMode) => void;
  showSettings: boolean;
  setShowSettings: (s: boolean) => void;
  tagFilter: string;
  setTagFilter: (t: string) => void;
  hybridWeight: number;
  setHybridWeight: (w: number) => void;
  minVectorScore: number;
  setMinVectorScore: (s: number) => void;
  handleSearch: () => void;
  resetSettings: () => void;
  // Projects
  projects: Project[];
  projectInfos: Map<string, ProjectInfo>;
  // Doc viewer
  viewingDoc: { projectId: string; filePath: string; content: string } | null;
  viewDocument: (projectId: string, filePath: string) => void;
  openFile: (project: Project, filePath: string) => void;
  closeDoc: () => void;
  // Tree
  expandedProjects: Set<string>;
  expandedFolders: Set<string>;
  expandedResults: Set<string>;
  projectDocs: Map<string, DocItem[]>;
  toggleProject: (projectId: string) => void;
  toggleFolder: (folderKey: string) => void;
  toggleResult: (filePath: string) => void;
  buildTree: (docs: DocItem[]) => TreeData;
  isRefreshing: boolean;
  refreshAllProjects: () => Promise<void>;
}

const modeLabels: Record<SearchMode, string> = {
  hybrid: "하이브리드",
  keyword: "키워드",
  vector: "벡터",
};

interface FolderNodeProps {
  node: TreeNode;
  projectId: string;
  depth: number;
  expandedFolders: Set<string>;
  toggleFolder: (key: string) => void;
  viewDocument: (projectId: string, filePath: string) => void;
  viewingDoc: { projectId: string; filePath: string; content: string } | null;
}

function FolderNodeView({
  node, projectId, depth,
  expandedFolders, toggleFolder,
  viewDocument, viewingDoc,
}: FolderNodeProps) {
  const folderKey = `${projectId}/${node.fullPath}`;
  const isOpen = expandedFolders.has(folderKey);
  const indent = depth * 12;

  return (
    <div>
      <div
        className="flex items-center gap-1 py-0.5 text-xs text-[var(--text-tertiary)] cursor-pointer hover:text-[var(--text-secondary)] transition-colors rounded-md hover:bg-[var(--bg-hover)]"
        style={{ paddingLeft: `${indent + 8}px` }}
        onClick={() => toggleFolder(folderKey)}
      >
        {isOpen ? <ChevronDown size={10} /> : <ChevronRight size={10} />}
        <FolderOpen size={12} className="flex-shrink-0" />
        <span className="truncate">{node.name}</span>
        <span className="ml-auto pr-2 text-[var(--text-tertiary)]">{node.docs.length}</span>
      </div>
      {isOpen && (
        <>
          {Array.from(node.subfolders.values()).map((child) => (
            <FolderNodeView
              key={child.fullPath}
              node={child}
              projectId={projectId}
              depth={depth + 1}
              expandedFolders={expandedFolders}
              toggleFolder={toggleFolder}
              viewDocument={viewDocument}
              viewingDoc={viewingDoc}
            />
          ))}
          {node.docs.map((doc) => {
            const isActive = viewingDoc?.filePath === doc.file_path;
            return (
              <div
                key={doc.id}
                className={cn(
                  "flex items-center gap-1 py-1 rounded-md cursor-pointer text-xs transition-colors duration-150",
                  isActive ? "bg-[var(--accent)] text-[var(--bg-primary)]" : "text-[var(--text-secondary)] hover:bg-[var(--bg-hover)]",
                )}
                style={{ paddingLeft: `${indent + 24}px`, paddingRight: "8px" }}
                onClick={() => viewDocument(projectId, doc.file_path)}
              >
                <FileText size={11} className="flex-shrink-0" />
                <span className="truncate">{doc.title || doc.file_path.split("/").pop()}</span>
              </div>
            );
          })}
        </>
      )}
    </div>
  );
}

export function SearchView(props: SearchViewProps) {
  const {
    query, setQuery, selectedProject, setSelectedProject,
    results, clearResults, searching,
    searchMode, setSearchMode, showSettings, setShowSettings,
    tagFilter, setTagFilter, hybridWeight, setHybridWeight,
    minVectorScore, setMinVectorScore, handleSearch, resetSettings,
    projects, projectInfos,
    viewingDoc, viewDocument, openFile, closeDoc,
    expandedProjects, expandedFolders, expandedResults, projectDocs,
    toggleProject, toggleFolder, toggleResult, buildTree,
    isRefreshing, refreshAllProjects,
  } = props;

  return (
    <div className="flex flex-col h-full">
      {/* Search bar + filters */}
      <div className="flex-shrink-0 px-4 py-3 border-b border-[var(--border)]">
        <div className="flex gap-2 mb-2">
          <Input
            sizing="md"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSearch()}
            placeholder="문서를 검색하세요..."
            className="flex-1"
          />
          <Button variant="primary" size="md" onClick={handleSearch} disabled={searching}>
            {searching ? "검색 중..." : "검색"}
          </Button>
          <IconButton active={showSettings} onClick={() => setShowSettings(!showSettings)}>
            <Settings2 size={16} />
          </IconButton>
          <IconButton
            onClick={refreshAllProjects}
            disabled={isRefreshing}
            title="프로젝트 및 문서 목록 새로고침"
          >
            <RefreshCw size={16} className={isRefreshing ? "animate-spin" : ""} />
          </IconButton>
        </div>

        {/* Filters */}
        <div className="flex items-center gap-3 flex-wrap">
          <div className="flex items-center gap-2">
            <span className="text-xs text-[var(--text-tertiary)]">프로젝트:</span>
            <Select value={selectedProject} onChange={(e) => setSelectedProject(e.target.value)}>
              <option value="">전체</option>
              {projects.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
            </Select>
          </div>

          <div className="w-px h-4 bg-[var(--border)]" />

          <div className="flex items-center gap-1">
            <span className="text-xs text-[var(--text-tertiary)]">모드:</span>
            {(["hybrid", "keyword", "vector"] as SearchMode[]).map((m) => (
              <button
                key={m}
                onClick={() => setSearchMode(m)}
                className={cn(
                  "px-2 py-0.5 rounded-md text-xs transition-all duration-150 cursor-pointer",
                  searchMode === m
                    ? "bg-[var(--accent)] text-[var(--bg-primary)] font-medium"
                    : "text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] border border-[var(--border)]",
                )}
              >
                {modeLabels[m]}
              </button>
            ))}
          </div>

          <div className="w-px h-4 bg-[var(--border)]" />

          <div className="flex items-center gap-1">
            <span className="text-xs text-[var(--text-tertiary)]">태그:</span>
            <Input
              sizing="sm"
              value={tagFilter}
              onChange={(e) => setTagFilter(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleSearch()}
              placeholder="rust, api"
              className="w-24"
            />
          </div>

          {results.length > 0 && (
            <>
              <div className="w-px h-4 bg-[var(--border)]" />
              <span className="text-xs text-[var(--text-tertiary)]">{results.length}건 검색됨</span>
              <button onClick={clearResults} className="text-xs text-[var(--text-tertiary)] hover:text-[var(--text-primary)] cursor-pointer transition-colors">초기화</button>
            </>
          )}
        </div>

        {/* Advanced settings */}
        {showSettings && (
          <Card className="mt-3 p-3">
            <div className="flex items-center justify-between mb-2">
              <span className="text-xs font-medium text-[var(--text-tertiary)]">검색 세부 설정</span>
              <Button variant="secondary" size="sm" onClick={resetSettings}>권장값으로 되돌리기</Button>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <div className="flex items-center justify-between mb-1">
                  <span className="text-xs text-[var(--text-tertiary)]">하이브리드 가중치 (벡터 비율)</span>
                  <span className="text-xs font-mono text-[var(--accent)]">{hybridWeight.toFixed(2)}</span>
                </div>
                <input type="range" min="0" max="1" step="0.05" value={hybridWeight}
                  onChange={(e) => setHybridWeight(parseFloat(e.target.value))}
                  className="w-full accent-[var(--accent)]" disabled={searchMode !== "hybrid"} />
                <div className="flex justify-between text-xs text-[var(--text-tertiary)] mt-0.5">
                  <span>키워드 중심</span><span>벡터 중심</span>
                </div>
              </div>
              <div>
                <div className="flex items-center justify-between mb-1">
                  <span className="text-xs text-[var(--text-tertiary)]">최소 벡터 유사도</span>
                  <span className="text-xs font-mono text-[var(--accent)]">{minVectorScore.toFixed(2)}</span>
                </div>
                <input type="range" min="0" max="1" step="0.01" value={minVectorScore}
                  onChange={(e) => setMinVectorScore(parseFloat(e.target.value))}
                  className="w-full accent-[var(--accent)]" disabled={searchMode === "keyword"} />
                <div className="flex justify-between text-xs text-[var(--text-tertiary)] mt-0.5">
                  <span>느슨하게</span><span>엄격하게</span>
                </div>
              </div>
            </div>
          </Card>
        )}
      </div>

      {/* Content: sidebar + viewer */}
      <div className="flex flex-1 min-h-0">
        {/* Left sidebar */}
        <div className="w-72 flex-shrink-0 border-r border-[var(--border)] overflow-y-auto p-3">
          {/* Search results grouped by file */}
          {results.length > 0 && (() => {
            const grouped = new Map<string, { projectId: string; projectName: string; filePath: string; items: SearchResult[] }>();
            for (const r of results) {
              if (!grouped.has(r.file_path)) {
                const project = projects.find((p) => p.name === r.project_name);
                grouped.set(r.file_path, { projectId: project?.id || "", projectName: r.project_name, filePath: r.file_path, items: [] });
              }
              grouped.get(r.file_path)!.items.push(r);
            }
            return (
              <div className="mb-3">
                <div className="text-xs font-medium text-[var(--text-tertiary)] mb-2">
                  {grouped.size}개 파일에서 {results.length}건
                </div>
                {Array.from(grouped.values()).map((group) => {
                  const isActive = viewingDoc?.filePath === group.filePath;
                  return (
                    <div key={group.filePath} className="mb-1">
                      <div
                        className={cn(
                          "px-2 py-1.5 rounded-md cursor-pointer transition-colors duration-150",
                          isActive ? "bg-[var(--accent)] text-[var(--bg-primary)]" : "hover:bg-[var(--bg-hover)]",
                        )}
                        onClick={() => viewDocument(group.projectId, group.filePath)}
                      >
                        <div className="flex items-center gap-1 text-xs">
                          <FileText size={12} className="flex-shrink-0" />
                          <span className="font-medium truncate">{group.filePath.split("/").pop()}</span>
                          <span className={cn("ml-auto flex-shrink-0", isActive ? "opacity-70" : "text-[var(--text-tertiary)]")}>{group.items.length}건</span>
                        </div>
                        <div className={cn("text-xs truncate", isActive ? "opacity-70" : "text-[var(--text-tertiary)]")}>
                          {group.projectName} / {group.filePath}
                        </div>
                      </div>
                      {group.items.length > 0 && (
                        <div className="flex items-center px-2 py-0.5">
                          <button
                            className="text-xs text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] flex items-center gap-1 cursor-pointer transition-colors"
                            onClick={(e) => { e.stopPropagation(); toggleResult(group.filePath); }}
                          >
                            {expandedResults.has(group.filePath) ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                            <span>매칭 섹션</span>
                          </button>
                        </div>
                      )}
                      {expandedResults.has(group.filePath) && group.items.map((r) => (
                        <div key={r.chunk_id}
                          className="px-3 py-1 text-xs text-[var(--text-tertiary)] cursor-pointer hover:text-[var(--text-secondary)] transition-colors"
                          onClick={() => viewDocument(group.projectId, group.filePath)}
                        >
                          {r.heading_path && <span>{r.heading_path}</span>}
                          <span className="text-[var(--text-tertiary)] ml-1">({(r.score * 100).toFixed(0)}%)</span>
                        </div>
                      ))}
                    </div>
                  );
                })}
                <div className="border-b border-[var(--border)] my-2" />
              </div>
            );
          })()}

          {/* Project tree */}
          <div className="text-xs font-medium text-[var(--text-tertiary)] mb-2">프로젝트</div>
          {projects.length === 0 && (
            <p className="text-xs text-[var(--text-tertiary)] px-2">프로젝트 탭에서 볼트를 추가하세요</p>
          )}
          {projects.map((p) => {
            const isExpanded = expandedProjects.has(p.id);
            const docs = projectDocs.get(p.id) || [];
            const tree = isExpanded ? buildTree(docs) : null;
            return (
              <div key={p.id} className="mb-1">
                <div
                  className={cn(
                    "flex items-center gap-1 px-2 py-1 rounded-md cursor-pointer transition-colors duration-150",
                    isExpanded ? "bg-[var(--bg-tertiary)]" : "hover:bg-[var(--bg-hover)]",
                  )}
                  onClick={() => toggleProject(p.id)}
                >
                  {isExpanded ? <ChevronDown size={12} className="text-[var(--text-tertiary)]" /> : <ChevronRight size={12} className="text-[var(--text-tertiary)]" />}
                  <span className="text-xs font-medium text-[var(--text-primary)]">{p.name}</span>
                  {projectInfos.get(p.id) && (
                    <span className="text-xs text-[var(--text-tertiary)] ml-auto">{projectInfos.get(p.id)!.stats.doc_count}</span>
                  )}
                </div>
                {isExpanded && tree && (
                  <div>
                    {Array.from(tree.subfolders.values()).map((node) => (
                      <FolderNodeView
                        key={node.fullPath}
                        node={node}
                        projectId={p.id}
                        depth={1}
                        expandedFolders={expandedFolders}
                        toggleFolder={toggleFolder}
                        viewDocument={viewDocument}
                        viewingDoc={viewingDoc}
                      />
                    ))}
                    {tree.rootDocs.map((doc) => {
                      const isActive = viewingDoc?.filePath === doc.file_path;
                      return (
                        <div key={doc.id}
                          className={cn(
                            "flex items-center gap-1 pl-6 pr-2 py-1 rounded-md cursor-pointer text-xs transition-colors duration-150",
                            isActive ? "bg-[var(--accent)] text-[var(--bg-primary)]" : "text-[var(--text-secondary)] hover:bg-[var(--bg-hover)]",
                          )}
                          onClick={() => viewDocument(p.id, doc.file_path)}
                        >
                          <FileText size={11} className="flex-shrink-0" />
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

        {/* Right: Document viewer */}
        <div className="flex-1 overflow-y-auto min-w-[400px]">
          {viewingDoc ? (
            <div className="p-6 overflow-x-auto">
              <div className="flex items-center justify-between mb-4">
                <span className="text-sm text-[var(--text-tertiary)] truncate mr-2">{viewingDoc.filePath}</span>
                <div className="flex gap-2">
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => viewDocument(viewingDoc.projectId, viewingDoc.filePath)}
                    title="문서 내용 새로고침"
                  >
                    <RefreshCw size={12} className="mr-1" /> 새로고침
                  </Button>
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={() => {
                      const project = projects.find((p) => p.id === viewingDoc.projectId);
                      if (project) openFile(project, viewingDoc.filePath);
                    }}
                  >
                    <ExternalLink size={12} className="mr-1" /> Obsidian에서 열기
                  </Button>
                  <Button variant="secondary" size="sm" onClick={closeDoc}>
                    <X size={12} className="mr-1" /> 닫기
                  </Button>
                </div>
              </div>
              <div className="prose prose-invert max-w-none text-sm text-[var(--text-primary)]">
                <Markdown remarkPlugins={[remarkGfm]}>{viewingDoc.content}</Markdown>
              </div>
            </div>
          ) : (
            <div className="flex items-center justify-center h-full">
              <div className="text-center">
                <Search size={40} className="mx-auto mb-3 text-[var(--text-tertiary)]" />
                <p className="text-base font-medium text-[var(--text-secondary)] mb-1">문서를 선택하세요</p>
                <p className="text-sm text-[var(--text-tertiary)]">왼쪽에서 프로젝트를 펼치거나 검색하여 문서를 확인할 수 있습니다</p>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
