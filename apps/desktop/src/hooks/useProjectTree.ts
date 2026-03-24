import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { DocItem } from "../types";

export interface TreeNode {
  name: string;
  fullPath: string;
  subfolders: Map<string, TreeNode>;
  docs: DocItem[];
}

export interface TreeData {
  subfolders: Map<string, TreeNode>;
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
    const rootDocs: DocItem[] = [];
    const subfolders: Map<string, TreeNode> = new Map();

    const getOrCreateNode = (
      map: Map<string, TreeNode>,
      name: string,
      fullPath: string,
    ): TreeNode => {
      if (!map.has(name)) {
        map.set(name, { name, fullPath, subfolders: new Map(), docs: [] });
      }
      return map.get(name)!;
    };

    for (const doc of docs) {
      const parts = doc.file_path.split("/");
      if (parts.length === 1) {
        rootDocs.push(doc);
      } else {
        const folderParts = parts.slice(0, -1);
        let currentMap = subfolders;
        for (let i = 0; i < folderParts.length; i++) {
          const name = folderParts[i];
          const fullPath = folderParts.slice(0, i + 1).join("/");
          const node = getOrCreateNode(currentMap, name, fullPath);
          if (i === folderParts.length - 1) {
            node.docs.push(doc);
          }
          currentMap = node.subfolders;
        }
      }
    }

    return { subfolders, rootDocs };
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
