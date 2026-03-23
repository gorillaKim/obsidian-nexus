import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AnimatePresence, motion } from "framer-motion";
import { Sidebar } from "./components/layout/Sidebar";
import { ChatPanel } from "./components/layout/ChatPanel";
import { DashboardView } from "./components/views/DashboardView";
import { SearchView } from "./components/views/SearchView";
import { ProjectsView } from "./components/views/ProjectsView";
import { GuideView } from "./components/views/GuideView";
import { SettingsView } from "./components/views/SettingsView";
import { useProjects } from "./hooks/useProjects";
import { useSearch } from "./hooks/useSearch";
import { useDocViewer } from "./hooks/useDocViewer";
import { useProjectTree } from "./hooks/useProjectTree";
import type { Tab, PopularDoc, TopProject } from "./types";

const MIN_CHAT_WIDTH = 280;
const MAX_CHAT_WIDTH = 640;

function App() {
  const [tab, setTab] = useState<Tab>("dashboard");
  const [chatOpen, setChatOpen] = useState(false);
  const [chatWidth, setChatWidth] = useState(360);

  const handleResizeStart = (e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = chatWidth;
    const onMove = (e: MouseEvent) => {
      const delta = startX - e.clientX;
      setChatWidth(Math.min(MAX_CHAT_WIDTH, Math.max(MIN_CHAT_WIDTH, startWidth + delta)));
    };
    const onUp = () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
    };
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  };

  const projectsHook = useProjects();
  const searchHook = useSearch();
  const docViewer = useDocViewer();
  const tree = useProjectTree();

  // 대시보드 인기 문서 상태
  const [dashLoading, setDashLoading] = useState(false);
  const [popularAll, setPopularAll] = useState<PopularDoc[]>([]);
  const [topProjects, setTopProjects] = useState<TopProject[]>([]);
  const [popularByProject, setPopularByProject] = useState<Map<string, PopularDoc[]>>(new Map());

  const loadDashboard = useCallback(async () => {
    setDashLoading(true);
    try {
      const [allDocs, tops] = await Promise.all([
        invoke<PopularDoc[]>("get_popular_documents", { limit: 10 }),
        invoke<TopProject[]>("get_top_projects", { limit: 2 }),
      ]);
      setPopularAll(allDocs);
      setTopProjects(tops);

      // 상위 2개 프로젝트별 랭킹 병렬 로드
      if (tops.length > 0) {
        const entries = await Promise.all(
          tops.map((p) =>
            invoke<PopularDoc[]>("get_popular_documents", { projectId: p.id, limit: 10 }).then(
              (docs) => [p.id, docs] as [string, PopularDoc[]]
            )
          )
        );
        setPopularByProject(new Map(entries));
      }
    } catch (e) {
      console.error("dashboard load failed", e);
    } finally {
      setDashLoading(false);
    }
  }, []);

  const handleTabChange = (newTab: Tab) => {
    setTab(newTab);
    docViewer.closeDoc();
    if (newTab === "dashboard") loadDashboard();
  };

  // 최초 마운트 시 대시보드 탭이 기본이면 로드
  useEffect(() => {
    if (tab === "dashboard") loadDashboard();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="h-screen flex overflow-hidden bg-[var(--bg-primary)]">
      {/* Sidebar */}
      <Sidebar
        activeTab={tab}
        onTabChange={handleTabChange}
        chatOpen={chatOpen}
        onChatToggle={() => setChatOpen(!chatOpen)}
      />

      {/* Content area */}
      <main className="flex-1 min-w-[480px] overflow-hidden">
        <AnimatePresence mode="wait">
          <motion.div
            key={tab}
            initial={{ opacity: 0, y: 6 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -6 }}
            transition={{ duration: 0.15 }}
            className="h-full overflow-y-auto overflow-x-auto"
          >
            {tab === "dashboard" && (
              <DashboardView
                totalProjects={projectsHook.projects.length}
                totalDocs={projectsHook.totalDocs}
                totalChunks={Array.from(projectsHook.projectInfos.values()).reduce(
                  (sum, info) => sum + info.stats.chunk_count, 0
                )}
                popularAll={popularAll}
                topProjects={topProjects}
                popularByProject={popularByProject}
                allProjects={projectsHook.projects.map((p) => ({ id: p.id, name: p.name }))}
                loading={dashLoading}
                onOpenFile={(projectId, filePath) => invoke("open_file", { projectId, filePath })}
              />
            )}

            {tab === "search" && (
              <SearchView
                query={searchHook.query}
                setQuery={searchHook.setQuery}
                selectedProject={searchHook.selectedProject}
                setSelectedProject={searchHook.setSelectedProject}
                results={searchHook.results}
                clearResults={searchHook.clearResults}
                searching={searchHook.searching}
                searchMode={searchHook.searchMode}
                setSearchMode={searchHook.setSearchMode}
                showSettings={searchHook.showSettings}
                setShowSettings={searchHook.setShowSettings}
                tagFilter={searchHook.tagFilter}
                setTagFilter={searchHook.setTagFilter}
                hybridWeight={searchHook.hybridWeight}
                setHybridWeight={searchHook.setHybridWeight}
                minVectorScore={searchHook.minVectorScore}
                setMinVectorScore={searchHook.setMinVectorScore}
                handleSearch={searchHook.handleSearch}
                resetSettings={searchHook.resetSettings}
                projects={projectsHook.projects}
                projectInfos={projectsHook.projectInfos}
                viewingDoc={docViewer.viewingDoc}
                viewDocument={docViewer.viewDocument}
                openFile={docViewer.openFile}
                closeDoc={docViewer.closeDoc}
                expandedProjects={tree.expandedProjects}
                expandedFolders={tree.expandedFolders}
                expandedResults={tree.expandedResults}
                projectDocs={tree.projectDocs}
                toggleProject={tree.toggleProject}
                toggleFolder={tree.toggleFolder}
                toggleResult={tree.toggleResult}
                buildTree={tree.buildTree}
                isRefreshing={tree.isRefreshing}
                refreshAllProjects={tree.refreshAllProjects}
              />
            )}

            {tab === "projects" && (
              <ProjectsView
                projects={projectsHook.projects}
                projectInfos={projectsHook.projectInfos}
                indexing={projectsHook.indexing}
                adding={projectsHook.adding}
                syncing={projectsHook.syncing}
                onIndex={projectsHook.handleIndex}
                onAddVault={projectsHook.handleAddVault}
                onSync={projectsHook.handleSync}
                onRemove={projectsHook.handleRemoveProject}
              />
            )}

            {tab === "guide" && <GuideView />}
            {tab === "settings" && <SettingsView />}
          </motion.div>
        </AnimatePresence>
      </main>

      {/* Chat panel */}
      <AnimatePresence>
        {chatOpen && (
          <ChatPanel
            onClose={() => setChatOpen(false)}
            currentProjectId={projectsHook.projects[0]?.id}
            currentProjectName={projectsHook.projects[0]?.name}
            width={chatWidth}
            onResizeStart={handleResizeStart}
          />
        )}
      </AnimatePresence>
    </div>
  );
}

export default App;
