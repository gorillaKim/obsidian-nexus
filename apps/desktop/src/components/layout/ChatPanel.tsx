import { useEffect, useRef, useState } from "react";
import { motion } from "framer-motion";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  X,
  MessageSquare,
  Plus,
  Send,
  Loader2,
  AlertCircle,
  Bot,
  User,
  RefreshCw,
} from "lucide-react";
import { IconButton } from "../ui/IconButton";
import { useChat } from "../../hooks/useChat";

interface ChatPanelProps {
  onClose: () => void;
  currentProjectId?: string;
  currentProjectName?: string;
  width?: number;
  onResizeStart?: (e: React.MouseEvent) => void;
}

export function ChatPanel({
  onClose,
  currentProjectId,
  currentProjectName,
  width = 360,
  onResizeStart,
}: ChatPanelProps) {
  const {
    agents,
    sessions,
    activeSession,
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
  } = useChat({
    projectId: currentProjectId,
    projectName: currentProjectName,
  });

  const [input, setInput] = useState("");
  const [showNewSession, setShowNewSession] = useState(false);
  const [selectedCli, setSelectedCli] = useState<string>("");
  const [selectedModel, setSelectedModel] = useState<string>("");
  const [initializing, setInitializing] = useState(true);
  const [editingSessionId, setEditingSessionId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    // Defer to next frame so the panel animation renders first
    const raf = requestAnimationFrame(() => {
      async function init() {
        await Promise.all([detectAgents(), loadSessions()]);
        setInitializing(false);
      }
      init();
    });
    return () => cancelAnimationFrame(raf);
  }, [detectAgents, loadSessions]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [activeMessages]);

  // 현재 Claude만 지원 (Gemini는 향후 확장)
  const supportedAgents = agents.filter((a) => a.cli === "claude");

  useEffect(() => {
    if (supportedAgents.length > 0 && !selectedCli) {
      setSelectedCli(supportedAgents[0].cli);
      setSelectedModel(supportedAgents[0].models[0] || "");
    }
  }, [supportedAgents, selectedCli]);

  const handleSend = async () => {
    const trimmed = input.trim();
    if (!trimmed || status === "generating") return;
    setInput("");
    await sendMessage(trimmed);
    inputRef.current?.focus();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleCreateSession = async () => {
    if (!selectedCli || !selectedModel) return;
    await createSession(
      selectedCli,
      selectedModel,
      currentProjectId || "default",
      currentProjectName ? `${currentProjectName} 사서` : undefined
    );
    setShowNewSession(false);
  };

  const selectedAgent = supportedAgents.find((a) => a.cli === selectedCli);

  const hasAgents = supportedAgents.length > 0;

  return (
    <motion.div
      initial={{ width: 0, opacity: 0 }}
      animate={{ width, opacity: 1 }}
      exit={{ width: 0, opacity: 0 }}
      transition={{ duration: 0.2, ease: "easeInOut" }}
      className="relative h-full border-l border-[var(--border)] bg-[var(--bg-secondary)] flex flex-col overflow-hidden flex-shrink-0"
    >
      {/* Resize handle */}
      <div
        className="absolute left-0 top-0 w-1 h-full cursor-col-resize z-10 hover:bg-[var(--accent)] opacity-0 hover:opacity-30 transition-opacity"
        onMouseDown={onResizeStart}
      />
      {/* Header */}
      <div className="flex items-center justify-between px-4 h-14 border-b border-[var(--border)] flex-shrink-0">
        <div className="flex items-center gap-2">
          <MessageSquare size={16} className="text-[var(--accent)]" />
          <span className="text-sm font-bold text-[var(--text-primary)]">
            Nexus 사서
          </span>
        </div>
        <div className="flex items-center gap-1">
          {hasAgents && (
            <IconButton onClick={() => setShowNewSession(!showNewSession)}>
              <Plus size={14} />
            </IconButton>
          )}
          <IconButton onClick={onClose}>
            <X size={16} />
          </IconButton>
        </div>
      </div>

      {/* Session list */}
      {sessions.length > 0 && (
        <div className="flex flex-col gap-0.5 px-2 py-1.5 border-b border-[var(--border)] flex-shrink-0 max-h-28 overflow-y-auto">
          {sessions.map((s) => (
            <div
              key={s.id}
              className={`flex items-center justify-between px-2 py-1 text-xs rounded-md cursor-pointer transition-colors ${
                activeSession?.id === s.id
                  ? "bg-[var(--accent-muted)] text-[var(--accent)]"
                  : "text-[var(--text-tertiary)] hover:bg-[var(--bg-tertiary)]"
              }`}
              onClick={() => switchSession(s.id)}
            >
              {editingSessionId === s.id ? (
                <input
                  autoFocus
                  className="flex-1 px-1 py-0 text-xs bg-[var(--bg-secondary)] border border-[var(--accent)] rounded outline-none text-[var(--text-primary)]"
                  value={editingName}
                  onChange={(e) => setEditingName(e.target.value)}
                  onBlur={() => {
                    renameSession(s.id, editingName);
                    setEditingSessionId(null);
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      renameSession(s.id, editingName);
                      setEditingSessionId(null);
                    }
                    if (e.key === "Escape") setEditingSessionId(null);
                  }}
                  onClick={(e) => e.stopPropagation()}
                />
              ) : (
                <span
                  className="truncate flex-1"
                  onDoubleClick={(e) => {
                    e.stopPropagation();
                    setEditingSessionId(s.id);
                    setEditingName(s.name);
                  }}
                >
                  {s.name} ({s.model})
                </span>
              )}
              <button
                className="ml-1 p-0.5 opacity-40 hover:opacity-100 transition-opacity"
                onClick={(e) => {
                  e.stopPropagation();
                  deleteSession(s.id);
                }}
              >
                <X size={10} />
              </button>
            </div>
          ))}
        </div>
      )}

      {/* New session form */}
      {showNewSession && hasAgents && (
        <div className="p-3 border-b border-[var(--border)] bg-[var(--bg-tertiary)] flex-shrink-0">
          <div className="flex gap-2 mb-2">
            <span className="flex items-center px-2 py-1.5 text-xs text-[var(--text-secondary)]">
              {selectedCli} (v{selectedAgent?.version})
            </span>
            <select
              value={selectedModel}
              onChange={(e) => setSelectedModel(e.target.value)}
              className="flex-1 px-2 py-1.5 text-xs rounded-md bg-[var(--bg-secondary)] border border-[var(--border)] text-[var(--text-primary)]"
            >
              {selectedAgent?.models.map((m) => (
                <option key={m} value={m}>
                  {m}
                </option>
              ))}
            </select>
          </div>
          <button
            onClick={handleCreateSession}
            className="w-full px-3 py-1.5 text-xs rounded-md bg-[var(--accent)] text-white hover:opacity-90 transition-opacity"
          >
            새 세션 시작
          </button>
        </div>
      )}

      {/* Content area */}
      <div className="flex-1 overflow-y-auto p-4">
        {initializing ? (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-center">
              <Loader2 size={24} className="animate-spin text-[var(--accent)] mx-auto mb-3" />
              <p className="text-sm text-[var(--text-tertiary)]">에이전트 감지 중...</p>
            </div>
          </div>
        ) : !hasAgents ? (
          /* Onboarding: No CLI detected */
          <OnboardingView onRetry={detectAgents} />
        ) : !activeSession ? (
          /* No active session */
          <EmptySessionView onCreate={() => setShowNewSession(true)} />
        ) : (
          /* Messages */
          <div className="space-y-4">
            {activeMessages.map((msg) => (
              <div
                key={msg.id}
                className={`flex gap-2 ${
                  msg.role === "user" ? "justify-end" : "justify-start"
                }`}
              >
                {msg.role === "assistant" && (
                  <div className="w-6 h-6 rounded-full bg-[var(--accent-muted)] flex items-center justify-center flex-shrink-0 mt-1">
                    <Bot size={12} className="text-[var(--accent)]" />
                  </div>
                )}
                <div
                  className={`max-w-[80%] px-3 py-2 rounded-lg text-sm ${
                    msg.role === "user"
                      ? "bg-[var(--accent)] text-white"
                      : "bg-[var(--bg-tertiary)] text-[var(--text-primary)]"
                  }`}
                >
                  {msg.role === "user" ? (
                    <p className="whitespace-pre-wrap">{msg.content}</p>
                  ) : (
                    <div className="prose-chat">
                      <ReactMarkdown
                        remarkPlugins={[remarkGfm]}
                        components={{
                          h1: ({ children }) => <h1 className="text-base font-bold mb-2 mt-1">{children}</h1>,
                          h2: ({ children }) => <h2 className="text-sm font-bold mb-1.5 mt-1">{children}</h2>,
                          h3: ({ children }) => <h3 className="text-sm font-semibold mb-1 mt-1">{children}</h3>,
                          p: ({ children }) => <p className="mb-2 last:mb-0 leading-relaxed">{children}</p>,
                          ul: ({ children }) => <ul className="list-disc list-inside mb-2 space-y-0.5">{children}</ul>,
                          ol: ({ children }) => <ol className="list-decimal list-inside mb-2 space-y-0.5">{children}</ol>,
                          li: ({ children }) => <li className="text-sm">{children}</li>,
                          code: ({ inline, children }: { inline?: boolean; children?: React.ReactNode }) =>
                            inline ? (
                              <code className="px-1 py-0.5 rounded text-xs font-mono bg-[var(--bg-primary)] text-[var(--accent)]">{children}</code>
                            ) : (
                              <code className="block px-3 py-2 rounded-md text-xs font-mono bg-[var(--bg-primary)] text-[var(--text-primary)] overflow-x-auto my-2">{children}</code>
                            ),
                          pre: ({ children }) => <>{children}</>,
                          blockquote: ({ children }) => <blockquote className="border-l-2 border-[var(--accent)] pl-3 my-2 text-[var(--text-secondary)] italic">{children}</blockquote>,
                          strong: ({ children }) => <strong className="font-semibold text-[var(--text-primary)]">{children}</strong>,
                          a: ({ href, children }) => <a href={href} className="text-[var(--accent)] underline hover:opacity-80">{children}</a>,
                          hr: () => <hr className="border-[var(--border)] my-3" />,
                        }}
                      >
                        {msg.content}
                      </ReactMarkdown>
                    </div>
                  )}
                </div>
                {msg.role === "user" && (
                  <div className="w-6 h-6 rounded-full bg-[var(--bg-tertiary)] flex items-center justify-center flex-shrink-0 mt-1">
                    <User size={12} className="text-[var(--text-secondary)]" />
                  </div>
                )}
              </div>
            ))}

            {/* Status indicator */}
            {status === "generating" && (
              <div className="flex items-center gap-2 text-xs text-[var(--text-tertiary)]">
                <Loader2 size={12} className="animate-spin" />
                <span>{toolInfo ? `${toolInfo} 실행 중...` : "답변 중..."}</span>
              </div>
            )}
            {status === "compacting" && (
              <div className="flex items-center gap-2 text-xs text-[var(--text-tertiary)]">
                <Loader2 size={12} className="animate-spin" />
                <span>사서가 기억을 정리 중...</span>
              </div>
            )}
            {status === "error" && error && (
              <div className="flex items-center gap-2 text-xs text-red-400">
                <AlertCircle size={12} />
                <span>{error}</span>
              </div>
            )}

            <div ref={messagesEndRef} />
          </div>
        )}
      </div>

      {/* Input */}
      {activeSession && (
        <div className="p-3 border-t border-[var(--border)] flex-shrink-0">
          <div className="flex gap-2">
            <input
              ref={inputRef}
              type="text"
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="사서에게 질문하세요..."
              disabled={status === "generating"}
              className="flex-1 px-3 py-2 text-sm rounded-lg bg-[var(--bg-tertiary)] border border-[var(--border)] text-[var(--text-primary)] placeholder:text-[var(--text-tertiary)] focus:outline-none focus:border-[var(--accent)] disabled:opacity-50"
            />
            {status === "generating" ? (
              <IconButton onClick={cancelMessage}>
                <X size={14} />
              </IconButton>
            ) : (
              <IconButton
                onClick={handleSend}
                disabled={!input.trim()}
              >
                <Send size={14} />
              </IconButton>
            )}
          </div>
        </div>
      )}
    </motion.div>
  );
}

function OnboardingView({ onRetry }: { onRetry: () => void }) {
  return (
    <div className="flex-1 flex items-center justify-center">
      <div className="text-center max-w-[280px]">
        <div className="w-12 h-12 rounded-xl bg-[var(--accent-muted)] flex items-center justify-center mx-auto mb-4">
          <AlertCircle size={24} className="text-[var(--accent)]" />
        </div>
        <p className="text-sm font-medium text-[var(--text-secondary)] mb-2">
          CLI 에이전트를 찾을 수 없습니다
        </p>
        <p className="text-xs text-[var(--text-tertiary)] mb-4">
          사서 기능을 사용하려면 Claude CLI 또는 Gemini CLI가 필요합니다.
        </p>
        <div className="space-y-2 text-xs text-left text-[var(--text-tertiary)] mb-4">
          <p>
            <strong>Claude CLI:</strong>{" "}
            <code className="text-[var(--accent)]">
              npm install -g @anthropic-ai/claude-code
            </code>
          </p>
          <p>
            <strong>Gemini CLI:</strong>{" "}
            <code className="text-[var(--accent)]">
              npm install -g @google/gemini-cli
            </code>
          </p>
        </div>
        <button
          onClick={onRetry}
          className="flex items-center gap-1.5 mx-auto px-3 py-1.5 text-xs rounded-md bg-[var(--accent)] text-white hover:opacity-90 transition-opacity"
        >
          <RefreshCw size={12} />
          다시 감지
        </button>
      </div>
    </div>
  );
}

function EmptySessionView({ onCreate }: { onCreate: () => void }) {
  return (
    <div className="flex-1 flex items-center justify-center">
      <div className="text-center">
        <div className="w-12 h-12 rounded-xl bg-[var(--accent-muted)] flex items-center justify-center mx-auto mb-4">
          <MessageSquare size={24} className="text-[var(--accent)]" />
        </div>
        <p className="text-sm font-medium text-[var(--text-secondary)] mb-1">
          새 세션을 시작하세요
        </p>
        <p className="text-xs text-[var(--text-tertiary)] mb-4">
          볼트 문서를 검색하고 분석하는 AI 사서입니다
        </p>
        <button
          onClick={onCreate}
          className="flex items-center gap-1.5 mx-auto px-3 py-1.5 text-xs rounded-md bg-[var(--accent)] text-white hover:opacity-90 transition-opacity"
        >
          <Plus size={12} />
          세션 시작
        </button>
      </div>
    </div>
  );
}
