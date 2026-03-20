import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Project } from "../types";

interface ViewingDoc {
  projectId: string;
  filePath: string;
  content: string;
}

export function useDocViewer() {
  const [viewingDoc, setViewingDoc] = useState<ViewingDoc | null>(null);

  const viewDocument = async (projectId: string, filePath: string) => {
    try {
      const content = await invoke<string>("get_document", { projectId, filePath });
      setViewingDoc({ projectId, filePath, content });
    } catch (e) { console.error(e); }
  };

  const openFile = async (project: Project, filePath: string) => {
    try { await invoke("open_file", { projectId: project.id, filePath }); }
    catch (e) { console.error(e); }
  };

  const closeDoc = () => setViewingDoc(null);

  return { viewingDoc, viewDocument, openFile, closeDoc };
}
