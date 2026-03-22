import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { DocItem } from "../types";

export interface TreeData {
  folders: Map<string, DocItem[]>;
  rootDocs: DocItem[];
}

export function useProjectTree() {
  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(new Set());
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set());
  const [expandedResults, setExpandedResults] = useState<Set<string>>(new Set());
  const [projectDocs, setProjectDocs] = useState<Map<string, DocItem[]>>(new Map());

  const [isRefreshing, setIsRefreshing] = useState(false);

  const refreshAllProjects = async () => {
    if (isRefreshing) return;
    setIsRefreshing(true);
    try {
      const updated = new Map<string, DocItem[]>();
      for (const projectId of expandedProjects) {
        const docs = await invoke<DocItem[]>("list_documents", { projectId });
        updated.set(projectId, docs);
      }
      setProjectDocs((prev) => new Map([...prev, ...updated]));
    } catch (e) {
      console.error(e);
    } finally {
      setIsRefreshing(false);
    }
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

  const toggleFolder = (folderKey: string) => {
    setExpandedFolders((prev) => {
      const next = new Set(prev);
      if (next.has(folderKey)) next.delete(folderKey);
      else next.add(folderKey);
      return next;
    });
  };

  const toggleResult = (filePath: string) => {
    setExpandedResults((prev) => {
      const next = new Set(prev);
      if (next.has(filePath)) next.delete(filePath);
      else next.add(filePath);
      return next;
    });
  };

  const buildTree = (docs: DocItem[]): TreeData => {
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

  return {
    expandedProjects,
    expandedFolders,
    expandedResults,
    projectDocs,
    isRefreshing,
    toggleProject,
    toggleFolder,
    toggleResult,
    buildTree,
    refreshAllProjects,
  };
}
