import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  LayoutDashboard,
  Search,
  FolderOpen,
  BookOpen,
  Settings,
  MessageSquare,
  PanelLeftClose,
  PanelLeft,
} from "lucide-react";
import { cn } from "../../lib/utils";
import type { Tab } from "../../types";

interface SidebarProps {
  activeTab: Tab;
  onTabChange: (tab: Tab) => void;
  chatOpen: boolean;
  onChatToggle: () => void;
}

const navItems: { tab: Tab; icon: typeof LayoutDashboard; label: string }[] = [
  { tab: "dashboard", icon: LayoutDashboard, label: "대시보드" },
  { tab: "search", icon: Search, label: "검색" },
  { tab: "projects", icon: FolderOpen, label: "프로젝트" },
  { tab: "guide", icon: BookOpen, label: "가이드" },
  { tab: "settings", icon: Settings, label: "설정" },
];

export function Sidebar({ activeTab, onTabChange, chatOpen, onChatToggle }: SidebarProps) {
  const [collapsed, setCollapsed] = useState(false);

  return (
    <motion.aside
      className={cn(
        "flex flex-col h-full border-r border-[var(--border)] bg-[var(--bg-secondary)]",
        "select-none",
      )}
      animate={{ width: collapsed ? 56 : 200 }}
      transition={{ duration: 0.2, ease: "easeInOut" }}
    >
      {/* Drag region + Logo */}
      <div
        className="flex items-center gap-2 px-3 h-14 border-b border-[var(--border)] flex-shrink-0"
        data-tauri-drag-region
      >
        <AnimatePresence mode="wait">
          {!collapsed && (
            <motion.span
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="text-sm font-bold text-[var(--accent)] truncate"
            >
              Obsidian Nexus
            </motion.span>
          )}
        </AnimatePresence>
        <button
          onClick={() => setCollapsed(!collapsed)}
          className="ml-auto p-1.5 rounded-md text-[var(--text-tertiary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-hover)] transition-colors cursor-pointer"
        >
          {collapsed ? <PanelLeft size={16} /> : <PanelLeftClose size={16} />}
        </button>
      </div>

      {/* Nav items */}
      <nav className="flex-1 py-2 px-2 space-y-0.5">
        {navItems.map(({ tab, icon: Icon, label }) => {
          const isActive = activeTab === tab;
          return (
            <button
              key={tab}
              onClick={() => onTabChange(tab)}
              className={cn(
                "w-full flex items-center gap-3 px-2.5 py-2 rounded-lg transition-all duration-150 cursor-pointer",
                isActive
                  ? "bg-[var(--accent-muted)] text-[var(--accent)] border-l-2 border-[var(--accent)]"
                  : "text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--bg-hover)]",
              )}
            >
              <Icon size={18} className="flex-shrink-0" />
              <AnimatePresence mode="wait">
                {!collapsed && (
                  <motion.span
                    initial={{ opacity: 0, width: 0 }}
                    animate={{ opacity: 1, width: "auto" }}
                    exit={{ opacity: 0, width: 0 }}
                    className="text-sm font-medium truncate"
                  >
                    {label}
                  </motion.span>
                )}
              </AnimatePresence>
            </button>
          );
        })}
      </nav>

      {/* Bottom: Chat toggle */}
      <div className="px-2 pb-3 space-y-0.5">
        <button
          onClick={onChatToggle}
          className={cn(
            "w-full flex items-center gap-3 px-2.5 py-2 rounded-lg transition-all duration-150 cursor-pointer",
            chatOpen
              ? "bg-[var(--accent-muted)] text-[var(--accent)]"
              : "text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--bg-hover)]",
          )}
        >
          <MessageSquare size={18} className="flex-shrink-0" />
          <AnimatePresence mode="wait">
            {!collapsed && (
              <motion.span
                initial={{ opacity: 0, width: 0 }}
                animate={{ opacity: 1, width: "auto" }}
                exit={{ opacity: 0, width: 0 }}
                className="text-sm font-medium truncate"
              >
                채팅
              </motion.span>
            )}
          </AnimatePresence>
        </button>
      </div>
    </motion.aside>
  );
}
