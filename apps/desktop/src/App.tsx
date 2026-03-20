import { useState } from "react";
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
import type { Tab } from "./types";

function App() {
  const [tab, setTab] = useState<Tab>("dashboard");
  const [chatOpen, setChatOpen] = useState(false);

  const projectsHook = useProjects();
  const searchHook = useSearch();
  const docViewer = useDocViewer();
  const tree = useProjectTree();

  const handleTabChange = (newTab: Tab) => {
    setTab(newTab);
    docViewer.closeDoc();
  };

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
      <main className="flex-1 min-w-0 overflow-hidden">
        <AnimatePresence mode="wait">
          <motion.div
            key={tab}
            initial={{ opacity: 0, y: 6 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -6 }}
            transition={{ duration: 0.15 }}
            className="h-full overflow-y-auto"
          >
            {tab === "dashboard" && (
              <DashboardView
                projects={projectsHook.projects}
                projectInfos={projectsHook.projectInfos}
                totalDocs={projectsHook.totalDocs}
                indexing={projectsHook.indexing}
                onIndex={projectsHook.handleIndex}
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
        {chatOpen && <ChatPanel onClose={() => setChatOpen(false)} />}
      </AnimatePresence>
    </div>
  );
}

export default App;
