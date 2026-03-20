import { useState, useTransition } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { SearchResult, SearchMode } from "../types";

export function useSearch() {
  const [query, setQuery] = useState("");
  const [selectedProject, setSelectedProject] = useState<string>("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [searchMode, setSearchMode] = useState<SearchMode>("hybrid");
  const [showSettings, setShowSettings] = useState(false);
  const [tagFilter, setTagFilter] = useState("");
  const [hybridWeight, setHybridWeight] = useState(0.7);
  const [minVectorScore, setMinVectorScore] = useState(0.65);
  const [, startTransition] = useTransition();

  const handleSearch = async () => {
    if (!query.trim()) return;
    setSearching(true);
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
      startTransition(() => {
        setResults(res);
      });
    } catch (e) { console.error(e); }
    setSearching(false);
  };

  const clearResults = () => setResults([]);

  const resetSettings = () => {
    setHybridWeight(0.7);
    setMinVectorScore(0.65);
  };

  return {
    query, setQuery,
    selectedProject, setSelectedProject,
    results, clearResults,
    searching,
    searchMode, setSearchMode,
    showSettings, setShowSettings,
    tagFilter, setTagFilter,
    hybridWeight, setHybridWeight,
    minVectorScore, setMinVectorScore,
    handleSearch,
    resetSettings,
  };
}
