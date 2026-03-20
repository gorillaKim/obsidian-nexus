import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { Project, ProjectInfo } from "../types";

export function useProjects() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [projectInfos, setProjectInfos] = useState<Map<string, ProjectInfo>>(new Map());
  const [indexing, setIndexing] = useState<Set<string>>(new Set());
  const [adding, setAdding] = useState(false);
  const [syncing, setSyncing] = useState<Set<string>>(new Set());

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
      const result = await invoke<{ added: number; projects: unknown[] }>("auto_add_vaults", { path: folderPath });
      if (result.added === 0) {
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

  const totalDocs = Array.from(projectInfos.values()).reduce((sum, i) => sum + i.stats.doc_count, 0);

  return {
    projects,
    projectInfos,
    indexing,
    adding,
    syncing,
    totalDocs,
    handleIndex,
    handleAddVault,
    handleSync,
    handleRemoveProject,
    loadProjects,
  };
}
