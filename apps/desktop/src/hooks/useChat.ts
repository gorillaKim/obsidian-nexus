import { useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  DetectedAgent,
  SessionMeta,
  ChatMessage,
  AgentStatus,
} from "../types";

/** Bridge response from sidecar via Tauri event */
interface BridgeResponse {
  type: string;
  sessionId: string;
  content?: string;
  model?: string;
  toolName?: string;
  input?: Record<string, unknown>;
  status?: string;
  done?: boolean;
  cost?: number;
  duration?: number;
  usage?: { input: number; output: number };
  code?: string;
  message?: string;
  retryable?: boolean;
}

interface UseChatOptions {
  projectId?: string;
  projectName?: string;
  projectPath?: string;
  docCount?: number;
  topTags?: string[];
}

export function useChat(options: UseChatOptions = {}) {
  const [agents, setAgents] = useState<DetectedAgent[]>([]);
  const [sessions, setSessions] = useState<SessionMeta[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<Record<string, ChatMessage[]>>({});
  const [status, setStatus] = useState<AgentStatus>("idle");
  const [toolInfo, setToolInfo] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Accumulate streaming text for current response
  const streamBuffer = useRef<string>("");

  // Listen for chat-stream events
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    listen<BridgeResponse>("chat-stream", (event) => {
      const msg = event.payload;
      if (!msg.sessionId) return;

      switch (msg.type) {
        case "init":
          setStatus("idle");
          break;

        case "thought":
          // Could display thinking indicator
          break;

        case "tool_use":
          if (msg.status === "running") {
            setStatus("generating");
            const inputStr = msg.input
              ? `(${Object.values(msg.input).join(", ")})`
              : "";
            setToolInfo(`${msg.toolName} ${inputStr}`);
          } else {
            setToolInfo(null);
          }
          break;

        case "text":
          setStatus("generating");
          setToolInfo(null);
          // Update the last assistant message with accumulated text
          streamBuffer.current = msg.content || "";
          updateLastAssistant(msg.sessionId, streamBuffer.current);
          break;

        case "result":
          setStatus("done");
          setToolInfo(null);
          // Final content
          if (msg.content) {
            updateLastAssistant(msg.sessionId, msg.content);
          }
          // Reset after brief delay
          setTimeout(() => setStatus("idle"), 500);
          break;

        case "error":
          setStatus("error");
          setToolInfo(null);
          setError(msg.message || "알 수 없는 에러");
          break;
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  function updateLastAssistant(sessionId: string, content: string) {
    setMessages((prev) => {
      const msgs = prev[sessionId] || [];
      const last = msgs[msgs.length - 1];
      if (last && last.role === "assistant") {
        // Update existing assistant message
        const updated = [...msgs];
        updated[updated.length - 1] = { ...last, content };
        return { ...prev, [sessionId]: updated };
      }
      return prev;
    });
  }

  const detectAgents = useCallback(async () => {
    try {
      const detected = await invoke<DetectedAgent[]>("detect_cli_agents");
      setAgents(detected);
      return detected;
    } catch (e) {
      setError(String(e));
      return [];
    }
  }, []);

  const loadSessions = useCallback(async () => {
    try {
      const list = await invoke<SessionMeta[]>("chat_list_sessions");
      setSessions(list);
      // 세션이 있고 아직 선택된 것이 없으면 첫 번째 자동 선택
      if (list.length > 0 && !activeSessionId) {
        setActiveSessionId(list[0].id);
      }
      return list;
    } catch (e) {
      setError(String(e));
      return [];
    }
  }, []);

  const createSession = useCallback(
    async (cli: string, model: string, projectId: string, name?: string) => {
      try {
        // Create session metadata
        const session = await invoke<SessionMeta>("chat_new_session", {
          cli,
          model,
          projectId,
          name: name || null,
        });
        setSessions((prev) => [...prev, session]);
        setActiveSessionId(session.id);
        setMessages((prev) => ({ ...prev, [session.id]: [] }));
        setError(null);

        // Start sidecar session
        await invoke("chat_start_session", {
          sessionId: session.id,
          model,
          projectName: options.projectName || "default",
          projectPath: options.projectPath || "",
          docCount: options.docCount || 0,
          topTags: options.topTags || [],
        });

        return session;
      } catch (e) {
        setError(String(e));
        return null;
      }
    },
    [options]
  );

  const deleteSession = useCallback(
    async (sessionId: string) => {
      // Always remove from UI, even if backend call fails
      setSessions((prev) => prev.filter((s) => s.id !== sessionId));
      setMessages((prev) => {
        const next = { ...prev };
        delete next[sessionId];
        return next;
      });
      if (activeSessionId === sessionId) {
        setActiveSessionId(null);
      }
      try {
        await invoke("chat_close_session", { sessionId });
      } catch {
        // Best-effort: metadata cleanup may have succeeded even if sidecar wasn't running
      }
    },
    [activeSessionId]
  );

  const renameSession = useCallback(
    async (sessionId: string, name: string) => {
      const finalName = name.trim() || "New Session";
      try {
        await invoke("chat_rename_session", { sessionId, name: finalName });
        setSessions((prev) =>
          prev.map((s) => (s.id === sessionId ? { ...s, name: finalName } : s))
        );
      } catch (e) {
        setError(String(e));
      }
    },
    []
  );

  const switchSession = useCallback((sessionId: string) => {
    setActiveSessionId(sessionId);
    setStatus("idle");
    setError(null);
    setToolInfo(null);
  }, []);

  const sendMessage = useCallback(
    async (content: string) => {
      if (!activeSessionId) return;

      // Add user message
      const userMsg: ChatMessage = {
        id: crypto.randomUUID(),
        role: "user",
        content,
        timestamp: Date.now(),
      };

      // Add placeholder assistant message for streaming
      const assistantMsg: ChatMessage = {
        id: crypto.randomUUID(),
        role: "assistant",
        content: "",
        timestamp: Date.now(),
      };

      setMessages((prev) => ({
        ...prev,
        [activeSessionId]: [
          ...(prev[activeSessionId] || []),
          userMsg,
          assistantMsg,
        ],
      }));

      setStatus("generating");
      setError(null);
      streamBuffer.current = "";

      try {
        await invoke("chat_send_message", {
          sessionId: activeSessionId,
          message: content,
        });
      } catch (e) {
        setStatus("error");
        setError(String(e));
      }
    },
    [activeSessionId]
  );

  const cancelMessage = useCallback(async () => {
    if (!activeSessionId) return;
    try {
      await invoke("chat_cancel", { sessionId: activeSessionId });
    } catch (e) {
      setError(String(e));
    }
  }, [activeSessionId]);

  const activeMessages = activeSessionId
    ? messages[activeSessionId] || []
    : [];

  const activeSession = sessions.find((s) => s.id === activeSessionId) || null;

  return {
    agents,
    sessions,
    activeSession,
    activeSessionId,
    activeMessages,
    status,
    toolInfo,
    error,
    detectAgents,
    loadSessions,
    createSession,
    deleteSession,
    renameSession,
    switchSession,
    sendMessage,
    cancelMessage,
  };
}
