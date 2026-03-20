import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card } from "../ui/Card";
import { Button } from "../ui/Button";
import { Badge } from "../ui/Badge";
import { UpdateChecker } from "../UpdateChecker";
import type { McpStatus } from "../../types";

export function SettingsView() {
  const [mcpStatuses, setMcpStatuses] = useState<McpStatus[]>([]);
  const [registering, setRegistering] = useState<string | null>(null);

  const loadStatus = useCallback(async () => {
    try {
      const statuses = await invoke<McpStatus[]>("mcp_status");
      setMcpStatuses(statuses);
    } catch (e) {
      console.error("Failed to load MCP status", e);
    }
  }, []);

  useEffect(() => { loadStatus(); }, [loadStatus]);

  const handleRegister = async (name: string) => {
    setRegistering(name);
    try {
      await invoke("mcp_register", { name });
      await loadStatus();
    } catch (e) {
      console.error("Failed to register MCP", e);
    }
    setRegistering(null);
  };

  return (
    <div className="p-6 max-w-2xl mx-auto">
      <h2 className="text-lg font-bold text-[var(--accent)] mb-4">설정</h2>

      <div className="mb-6">
        <UpdateChecker variant="settings" />
      </div>

      <Card className="mb-4">
        <h3 className="font-medium text-[var(--text-primary)] mb-2">MCP 서버 자동 등록</h3>
        <p className="text-sm text-[var(--text-tertiary)] mb-4">
          AI 도구에 Obsidian Nexus MCP 서버를 등록하면, 에이전트가 볼트 문서를 직접 검색하고 읽을 수 있습니다.
        </p>
        <div className="space-y-2">
          {mcpStatuses.map((s) => (
            <div key={s.name} className="flex items-center justify-between p-3 rounded-lg bg-[var(--bg-primary)] border border-[var(--border)]">
              <div className="flex items-center gap-3">
                <span className="text-sm font-medium text-[var(--text-primary)]">{s.name}</span>
                {!s.installed && <Badge variant="muted">미설치</Badge>}
                {s.installed && s.registered && <Badge variant="success">등록됨</Badge>}
                {s.installed && !s.registered && <Badge variant="danger">미등록</Badge>}
              </div>
              {s.installed && !s.registered && (
                <Button
                  variant="primary"
                  size="sm"
                  onClick={() => handleRegister(s.name)}
                  disabled={registering === s.name}
                >
                  {registering === s.name ? "등록 중..." : "등록"}
                </Button>
              )}
            </div>
          ))}
          {mcpStatuses.length === 0 && (
            <p className="text-sm text-[var(--text-tertiary)]">MCP 대상 도구를 확인 중...</p>
          )}
        </div>
      </Card>

      <Card>
        <h3 className="font-medium text-[var(--text-primary)] mb-2">지원 도구</h3>
        <ul className="text-sm text-[var(--text-tertiary)] space-y-1">
          <li>• <strong className="text-[var(--text-secondary)]">Claude Desktop</strong> — Anthropic 데스크톱 앱</li>
          <li>• <strong className="text-[var(--text-secondary)]">Claude Code</strong> — CLI 기반 코딩 에이전트</li>
          <li>• <strong className="text-[var(--text-secondary)]">Gemini CLI</strong> — Google Gemini CLI 도구</li>
        </ul>
        <p className="text-xs text-[var(--text-tertiary)] mt-3">
          등록 후 해당 AI 도구를 재시작해야 MCP 서버가 인식됩니다.
        </p>
      </Card>
    </div>
  );
}
