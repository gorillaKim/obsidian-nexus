import { motion } from "framer-motion";
import { X, MessageSquare } from "lucide-react";
import { IconButton } from "../ui/IconButton";

interface ChatPanelProps {
  onClose: () => void;
}

export function ChatPanel({ onClose }: ChatPanelProps) {
  return (
    <motion.div
      initial={{ width: 0, opacity: 0 }}
      animate={{ width: 360, opacity: 1 }}
      exit={{ width: 0, opacity: 0 }}
      transition={{ duration: 0.2, ease: "easeInOut" }}
      className="h-full border-l border-[var(--border)] bg-[var(--bg-secondary)] flex flex-col overflow-hidden"
    >
      {/* Header */}
      <div className="flex items-center justify-between px-4 h-14 border-b border-[var(--border)] flex-shrink-0">
        <div className="flex items-center gap-2">
          <MessageSquare size={16} className="text-[var(--accent)]" />
          <span className="text-sm font-bold text-[var(--text-primary)]">Nexus Chat</span>
        </div>
        <IconButton onClick={onClose}>
          <X size={16} />
        </IconButton>
      </div>

      {/* Empty state */}
      <div className="flex-1 flex items-center justify-center p-6">
        <div className="text-center">
          <div className="w-12 h-12 rounded-xl bg-[var(--accent-muted)] flex items-center justify-center mx-auto mb-4">
            <MessageSquare size={24} className="text-[var(--accent)]" />
          </div>
          <p className="text-sm font-medium text-[var(--text-secondary)] mb-1">
            AI 채팅 준비 중
          </p>
          <p className="text-xs text-[var(--text-tertiary)]">
            볼트 문서를 기반으로 질문하고 답변을 받을 수 있습니다
          </p>
        </div>
      </div>

      {/* Input (disabled placeholder) */}
      <div className="p-3 border-t border-[var(--border)]">
        <div className="flex gap-2">
          <input
            type="text"
            disabled
            placeholder="준비 중..."
            className="flex-1 px-3 py-2 text-sm rounded-lg bg-[var(--bg-tertiary)] border border-[var(--border)] text-[var(--text-tertiary)] cursor-not-allowed"
          />
        </div>
      </div>
    </motion.div>
  );
}
