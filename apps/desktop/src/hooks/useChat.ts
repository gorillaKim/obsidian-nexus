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
  // Per-session status and toolInfo
  const [sessionStatus, setSessionStatus] = useState<Record<string, AgentStatus>>({});
  const [sessionToolInfo, setSessionToolInfo] = useState<Record<string, string | null>>({});
  const [error, setError] = useState<string | null>(null);

  const setStatus = (sid: string, s: AgentStatus) =>
    setSessionStatus((prev) => ({ ...prev, [sid]: s }));
  const setToolInfo = (sid: string, t: string | null) =>
    setSessionToolInfo((prev) => ({ ...prev, [sid]: t }));

  const activeSessionIdRef = useRef<string | null>(null);
  useEffect(() => {
    activeSessionIdRef.current = activeSessionId;
  }, [activeSessionId]);

  // Incremental listener map: sessionId → unlisten fn
  // Never fully torn down — only adds new sessions, removes deleted ones.
  const listenersRef = useRef<Map<string, UnlistenFn>>(new Map());

  // Incrementally manage per-session listeners.
  // On sessions change: subscribe to NEW sessions, unsubscribe from REMOVED sessions.
  // Listeners are never all torn down at once — no gap where events can be missed.
  useEffect(() => {
    const listeners = listenersRef.current;
    const currentIds = new Set(sessions.map((s) => s.id));

    // Remove listeners for sessions that no longer exist
    for (const [sid, unlisten] of listeners) {
      if (!currentIds.has(sid)) {
        unlisten();
        listeners.delete(sid);
      }
    }

    // Add listeners for new sessions
    const newSessions = sessions.filter((s) => !listeners.has(s.id));
    if (newSessions.length === 0) return;

    const promises = newSessions.map((session) => {
      const sid = session.id;
      return listen<BridgeResponse>(`chat-stream:${sid}`, (event) => {
        const msg = event.payload;
        const isActive = activeSessionIdRef.current === sid;

        switch (msg.type) {
          case "thought":
            break;

          case "tool_use":
            if (msg.status === "running") {
              setStatus(sid, "generating");
              const inputStr = msg.input
                ? `(${Object.values(msg.input).join(", ")})`
                : "";
              setToolInfo(sid, `${msg.toolName} ${inputStr}`);
            } else {
              setToolInfo(sid, null);
            }
            break;

          case "text":
            setStatus(sid, "generating");
            setToolInfo(sid, null);
            updateLastAssistant(sid, msg.content || "");
            break;

          case "result":
            setStatus(sid, "done");
            setToolInfo(sid, null);
            setTimeout(() => setStatus(sid, "idle"), 500);
            if (msg.content) {
              updateLastAssistant(sid, msg.content);
            }
            break;

          case "cancelled":
            setStatus(sid, "idle");
            setToolInfo(sid, null);
            break;

          case "error":
            setStatus(sid, "error");
            setToolInfo(sid, null);
            if (isActive) {
              setError(msg.message || "알 수 없는 에러");
            }
            break;
        }
      });
    });

    Promise.all(promises).then((fns) => {
      fns.forEach((fn, i) => {
        listeners.set(newSessions[i].id, fn);
      });
    });
  }, [sessions]);

  // RAF-batched streaming updates: coalesce rapid text events into one render per frame
  const pendingStreamRef = useRef<Map<string, string>>(new Map());
  const rafIdRef = useRef<number | null>(null);

  function flushStreamUpdates() {
    rafIdRef.current = null;
    const pending = pendingStreamRef.current;
    if (pending.size === 0) return;
    const snapshot = new Map(pending);
    pending.clear();
    setMessages((prev) => {
      let next = prev;
      for (const [sid, content] of snapshot) {
        const msgs = next[sid] || [];
        const last = msgs[msgs.length - 1];
        if (last && last.role === "assistant") {
          const updated = [...msgs];
          updated[updated.length - 1] = { ...last, content };
          next = { ...next, [sid]: updated };
        }
      }
      return next;
    });
  }

  function updateLastAssistant(sessionId: string, content: string) {
    pendingStreamRef.current.set(sessionId, content);
    if (rafIdRef.current === null) {
      rafIdRef.current = requestAnimationFrame(flushStreamUpdates);
    }
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

  // Destructure primitive values to avoid infinite loop:
  // passing `options` object directly causes useCallback to re-create on every render
  // because a new object literal ({}) !== ({}) even with same values.
  const { projectName, projectPath, docCount, topTags } = options;

  const loadSessions = useCallback(async () => {
    try {
      const list = await invoke<SessionMeta[]>("chat_list_sessions");
      setSessions(list);
      // 세션이 있고 아직 선택된 것이 없으면 첫 번째 자동 선택
      if (list.length > 0 && !activeSessionId) {
        const first = list[0];
        setActiveSessionId(first.id);
        // 앱 재시작 후 복원된 세션은 sidecar에 등록되어 있지 않으므로 start 요청 전송
        try {
          await invoke("chat_start_session", {
            sessionId: first.id,
            model: first.model,
            projectName: projectName || "default",
            projectPath: projectPath || "",
            docCount: docCount || 0,
            topTags: topTags || [],
          });
        } catch {
          // best-effort: 이미 실행 중이거나 sidecar 미실행 상태
        }
      }
      return list;
    } catch (e) {
      setError(String(e));
      return [];
    }
  }, [projectName, projectPath, docCount, topTags]);

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
          projectName: projectName || "default",
          projectPath: projectPath || "",
          docCount: docCount || 0,
          topTags: topTags || [],
        });

        return session;
      } catch (e) {
        setError(String(e));
        return null;
      }
    },
    [projectName, projectPath, docCount, topTags]
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
      setSessionStatus((prev) => {
        const next = { ...prev };
        delete next[sessionId];
        return next;
      });
      setSessionToolInfo((prev) => {
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

  const switchSession = useCallback(async (sessionId: string, session?: SessionMeta) => {
    setActiveSessionId(sessionId);
    setError(null);
    // 세션 전환 시 sidecar에 start 요청 (이미 등록되어 있으면 무시됨)
    if (session) {
      try {
        await invoke("chat_start_session", {
          sessionId,
          model: session.model,
          projectName: projectName || "default",
          projectPath: projectPath || "",
          docCount: docCount || 0,
          topTags: topTags || [],
        });
      } catch {
        // best-effort
      }
    }
  }, [options]);

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

      setStatus(activeSessionId, "generating");
      setError(null);

      try {
        await invoke("chat_send_message", {
          sessionId: activeSessionId,
          message: content,
        });
      } catch (e) {
        setStatus(activeSessionId, "error");
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
  const status: AgentStatus = (activeSessionId ? sessionStatus[activeSessionId] : null) ?? "idle";
  const toolInfo: string | null = (activeSessionId ? sessionToolInfo[activeSessionId] : null) ?? null;

  return {
    agents,
    sessions,
    activeSession,
    activeSessionId,
    activeMessages,
    status,
    toolInfo,
    error,
    sessionStatus,
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
