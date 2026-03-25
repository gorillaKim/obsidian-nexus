import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { RefreshCw, CheckCircle, XCircle, AlertCircle, ExternalLink, FlaskConical, HelpCircle, FolderOpen } from "lucide-react";
import { Card } from "../ui/Card";
import { Button } from "../ui/Button";
import { UpdateChecker } from "../UpdateChecker";
import type { SystemStatus, CliDiagnostics } from "../../types";

interface TestResult {
  ok: boolean;
  message: string;
}

interface OnboardStep {
  name: string;
  status: "created" | "skipped" | "error";
  message: string;
}

const INSTALL_URLS: Record<string, string> = {
  claude: "https://claude.ai/download",
  gemini: "https://github.com/google-gemini/gemini-cli",
  ollama: "https://ollama.com/download",
  obsidian: "https://obsidian.md",
};

function StatusIcon({ ok, warn }: { ok: boolean; warn?: boolean }) {
  if (ok) return <CheckCircle size={16} className="text-green-500 shrink-0" />;
  if (warn) return <AlertCircle size={16} className="text-yellow-500 shrink-0" />;
  return <XCircle size={16} className="text-red-400 shrink-0" />;
}

export function SettingsView() {
  const [status, setStatus] = useState<SystemStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [registering, setRegistering] = useState<string | null>(null);
  const [testing, setTesting] = useState<string | null>(null);
  const [testResults, setTestResults] = useState<Record<string, TestResult>>({});
  const [diagnosing, setDiagnosing] = useState<string | null>(null);
  const [diagResults, setDiagResults] = useState<Record<string, CliDiagnostics>>({});
  const [updating, setUpdating] = useState<string | null>(null);
  const [updateResults, setUpdateResults] = useState<Record<string, TestResult>>({});
  const [onboardPath, setOnboardPath] = useState("");
  const [onboarding, setOnboarding] = useState(false);
  const [onboardResults, setOnboardResults] = useState<OnboardStep[] | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const s = await invoke<SystemStatus>("system_status");
      setStatus(s);
    } catch (e) {
      console.error("system_status failed", e);
    }
    setLoading(false);
  }, []);

  useEffect(() => { load(); }, [load]);

  const handleRegister = async (name: string) => {
    setRegistering(name);
    try {
      await invoke("mcp_register", { name });
      await load();
    } catch (e) {
      console.error("mcp_register failed", e);
    }
    setRegistering(null);
  };

  const openUrl = (url: string) => invoke("open_url", { url });

  const handleDiagnose = async (cli: string) => {
    setDiagnosing(cli);
    try {
      const result = await invoke<CliDiagnostics>("diagnose_cli", { cli });
      setDiagResults((prev) => ({ ...prev, [cli]: result }));
    } catch (e) {
      console.error("diagnose_cli failed", e);
    }
    setDiagnosing(null);
  };

  const handlePickFolder = async () => {
    const selected = await openDialog({ directory: true, multiple: false, title: "온보딩할 프로젝트 폴더 선택" });
    if (typeof selected === "string") setOnboardPath(selected);
  };

  const handleOnboard = async () => {
    if (!onboardPath.trim()) return;
    setOnboarding(true);
    setOnboardResults(null);
    try {
      const steps = await invoke<OnboardStep[]>("run_onboard", { projectPath: onboardPath.trim() });
      setOnboardResults(steps);
    } catch (e) {
      setOnboardResults([{ name: "error", status: "error", message: String(e) }]);
    }
    setOnboarding(false);
  };

  const handleUpdate = async (key: string, command: string) => {
    setUpdating(key);
    setUpdateResults((prev) => { const n = { ...prev }; delete n[key]; return n; });
    try {
      const result = await invoke<TestResult>(command);
      setUpdateResults((prev) => ({ ...prev, [key]: result }));
    } catch (e) {
      setUpdateResults((prev) => ({ ...prev, [key]: { ok: false, message: String(e) } }));
    }
    setUpdating(null);
  };

  const handleTest = async (key: string, command: string, args?: Record<string, string>) => {
    setTesting(key);
    try {
      const result = await invoke<TestResult>(command, args);
      setTestResults((prev) => ({ ...prev, [key]: result }));
    } catch (e) {
      setTestResults((prev) => ({ ...prev, [key]: { ok: false, message: String(e) } }));
    }
    setTesting(null);
  };

  if (loading) {
    return (
      <div className="p-6 max-w-2xl mx-auto flex items-center gap-2 text-[var(--text-tertiary)]">
        <RefreshCw size={16} className="animate-spin" /> 상태 확인 중...
      </div>
    );
  }

  if (!status) return null;

  return (
    <div className="p-6 max-w-2xl mx-auto space-y-4">
      <div className="mb-2">
        <UpdateChecker variant="settings" />
      </div>

      {/* Nexus 바이너리 */}
      <Card>
        <h3 className="font-medium text-[var(--text-primary)] mb-3">Nexus 바이너리</h3>
        <div className="space-y-2">
          {[
            { key: "mcp", label: "nexus-mcp-server", s: status.mcp_binary, testCmd: "test_mcp", testArgs: undefined, updateCmd: "update_mcp_server" },
            { key: "obs", label: "obs-nexus", s: status.obs_nexus_binary, testCmd: "test_cli", testArgs: { cli: "obs-nexus" }, updateCmd: "update_obs_nexus" },
          ].map(({ key, label, s, testCmd, testArgs, updateCmd }) => (
            <div key={key}>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2 min-w-0">
                  <StatusIcon ok={s.installed} />
                  <span className="text-sm text-[var(--text-primary)] whitespace-nowrap">{label}</span>
                  {s.detail && (
                    <span className="text-xs text-[var(--text-tertiary)] truncate">{s.detail}</span>
                  )}
                  {!s.installed && <span className="text-xs text-red-400 whitespace-nowrap">앱을 재설치해주세요</span>}
                </div>
                {s.installed && (
                  <div className="flex items-center gap-1 shrink-0 ml-2">
                    <Button variant="ghost" size="sm" onClick={() => handleTest(key, testCmd, testArgs)} disabled={testing === key} title="테스트">
                      {testing === key ? <RefreshCw size={12} className="animate-spin" /> : <FlaskConical size={12} />}
                    </Button>
                    <Button variant="ghost" size="sm" onClick={() => handleUpdate(key, updateCmd)} disabled={updating === key} title="최신 버전으로 업데이트">
                      <RefreshCw size={12} className={updating === key ? "animate-spin" : ""} />
                    </Button>
                  </div>
                )}
              </div>
              {updateResults[key] && (
                <div className={`mt-1 text-xs px-2 py-1 rounded flex items-center gap-1 ${updateResults[key].ok ? "text-green-500 bg-green-500/10" : "text-red-400 bg-red-400/10"}`}>
                  {updateResults[key].ok ? <CheckCircle size={12} /> : <XCircle size={12} />}
                  {updateResults[key].message}
                </div>
              )}
              {testResults[key] && (
                <div className={`mt-1 text-xs px-2 py-1 rounded flex items-center gap-1 ${testResults[key].ok ? "text-green-500 bg-green-500/10" : "text-red-400 bg-red-400/10"}`}>
                  {testResults[key].ok ? <CheckCircle size={12} /> : <XCircle size={12} />}
                  {testResults[key].message}
                </div>
              )}
            </div>
          ))}
        </div>
      </Card>

      {/* MCP 연동 상태 */}
      <Card>
        <h3 className="font-medium text-[var(--text-primary)] mb-3">MCP 연동 상태</h3>
        <p className="text-xs text-[var(--text-tertiary)] mb-3">
          AI 도구에 Nexus MCP 서버가 등록되어야 에이전트가 볼트를 검색할 수 있습니다.
        </p>
        <div className="space-y-2">
          {status.mcp_registrations.map((s) => (
            <div key={s.name} className="flex items-center justify-between p-3 rounded-lg bg-[var(--bg-primary)] border border-[var(--border)]">
              <div className="flex items-center gap-2">
                <StatusIcon ok={s.installed && s.registered} warn={s.installed && !s.registered} />
                <span className="text-sm font-medium text-[var(--text-primary)]">{s.name}</span>
                <span className="text-xs text-[var(--text-tertiary)]">
                  {!s.installed ? "미설치" : s.registered ? "등록됨" : "미등록"}
                </span>
              </div>
              {s.installed && !s.registered && (
                <Button variant="primary" size="sm" onClick={() => handleRegister(s.name)} disabled={registering === s.name}>
                  {registering === s.name ? "등록 중..." : "등록"}
                </Button>
              )}
            </div>
          ))}
        </div>
      </Card>

      {/* CLI 에이전트 */}
      <Card>
        <div className="flex items-center gap-2 mb-3">
          <h3 className="font-medium text-[var(--text-primary)]">CLI 에이전트</h3>
          <div className="relative group">
            <span className="text-[var(--text-tertiary)] cursor-help">
              <HelpCircle size={14} />
            </span>
            <div className="absolute left-0 top-6 z-10 hidden group-hover:block w-72 p-3 rounded-lg text-xs text-[var(--text-secondary)] bg-[var(--bg-secondary)] border border-[var(--border)] shadow-lg">
              사서 기능은 <strong>Claude CLI</strong> 또는 <strong>Gemini CLI</strong>를 통해 동작합니다.<br /><br />
              CLI가 설치·인증된 상태여야 에이전트 채팅이 가능합니다.<br /><br />
              • Claude: <code className="bg-[var(--bg-primary)] px-1 rounded">npm install -g @anthropic-ai/claude-code</code><br />
              • Gemini: <code className="bg-[var(--bg-primary)] px-1 rounded">npm install -g @google/gemini-cli</code>
            </div>
          </div>
        </div>
        <div className="space-y-2">
          {status.cli_agents.map((a) => (
            <div key={a.cli}>
              <div className="flex items-center justify-between p-3 rounded-lg bg-[var(--bg-primary)] border border-[var(--border)]">
                <div className="flex items-center gap-2">
                  <StatusIcon ok={a.installed && a.authenticated} warn={a.installed && !a.authenticated} />
                  <span className="text-sm font-medium text-[var(--text-primary)] capitalize">{a.cli}</span>
                  {a.version && <span className="text-xs text-[var(--text-tertiary)]">v{a.version}</span>}
                  {a.installed && !a.authenticated && (
                    <span className="text-xs text-yellow-500">로그인 필요</span>
                  )}
                  {!a.installed && <span className="text-xs text-[var(--text-tertiary)]">미설치</span>}
                </div>
                <div className="flex items-center gap-2">
                  {a.installed && (
                    <Button variant="ghost" size="sm" onClick={() => handleTest(`cli_${a.cli}`, "test_cli", { cli: a.cli })} disabled={testing === `cli_${a.cli}`}>
                      {testing === `cli_${a.cli}` ? <RefreshCw size={12} className="animate-spin mr-1" /> : <FlaskConical size={12} className="mr-1" />}
                      테스트
                    </Button>
                  )}
                  {!a.installed && (
                    <>
                      <Button variant="ghost" size="sm" onClick={() => handleDiagnose(a.cli)} disabled={diagnosing === a.cli}>
                        {diagnosing === a.cli ? <RefreshCw size={12} className="animate-spin mr-1" /> : <FlaskConical size={12} className="mr-1" />}
                        진단
                      </Button>
                      <Button variant="ghost" size="sm" onClick={() => openUrl(INSTALL_URLS[a.cli])}>
                        <ExternalLink size={12} className="mr-1" /> 설치
                      </Button>
                    </>
                  )}
                </div>
              </div>
              {!a.installed && a.failure_reason && !diagResults[a.cli] && (
                <div className="mt-1 text-xs px-2 py-1 rounded flex items-center gap-1 text-[var(--text-tertiary)] bg-[var(--bg-secondary)]">
                  <AlertCircle size={12} className="shrink-0" />
                  {a.failure_reason}
                </div>
              )}
              {diagResults[a.cli] && (
                <div className="mt-2 text-xs rounded border border-[var(--border)] bg-[var(--bg-secondary)] p-2 space-y-1 font-mono">
                  {[
                    ["which", diagResults[a.cli].which_result],
                    ["직접 실행 (exit)", diagResults[a.cli].direct_exec_exit],
                    ["직접 실행 stdout", diagResults[a.cli].direct_exec_stdout],
                    ["직접 실행 stderr", diagResults[a.cli].direct_exec_stderr],
                    ["shell 실행 (exit)", diagResults[a.cli].shell_exec_exit],
                    ["shell 실행 stdout", diagResults[a.cli].shell_exec_stdout],
                    ["shell 실행 stderr", diagResults[a.cli].shell_exec_stderr],
                    ["nvm 경로", diagResults[a.cli].nvm_path],
                    ["nvm 실행 (exit)", diagResults[a.cli].nvm_exec_exit],
                    ["nvm 실행 stdout", diagResults[a.cli].nvm_exec_stdout],
                    ["find_cli_path 결과", diagResults[a.cli].find_cli_path_result],
                  ].map(([label, val]) => val ? (
                    <div key={label} className="flex gap-2">
                      <span className="text-[var(--text-tertiary)] shrink-0 w-36">{label}</span>
                      <span className="text-[var(--text-secondary)] break-all">{val}</span>
                    </div>
                  ) : null)}
                </div>
              )}
              {testResults[`cli_${a.cli}`] && (
                <div className={`mt-1 text-xs px-2 py-1 rounded flex items-center gap-1 ${testResults[`cli_${a.cli}`].ok ? "text-green-500 bg-green-500/10" : "text-red-400 bg-red-400/10"}`}>
                  {testResults[`cli_${a.cli}`].ok ? <CheckCircle size={12} /> : <XCircle size={12} />}
                  {testResults[`cli_${a.cli}`].message}
                </div>
              )}
            </div>
          ))}
        </div>
      </Card>

      {/* Ollama */}
      <Card>
        <h3 className="font-medium text-[var(--text-primary)] mb-3">Ollama (벡터 검색)</h3>
        <div className="flex items-center justify-between p-3 rounded-lg bg-[var(--bg-primary)] border border-[var(--border)]">
          <div className="flex items-center gap-2">
            <StatusIcon
              ok={status.ollama.installed && status.ollama.detail === "실행 중"}
              warn={status.ollama.installed && status.ollama.detail !== "실행 중"}
            />
            <span className="text-sm font-medium text-[var(--text-primary)]">Ollama</span>
            {status.ollama.detail && (
              <span className="text-xs text-[var(--text-tertiary)]">{status.ollama.detail}</span>
            )}
            {!status.ollama.installed && <span className="text-xs text-[var(--text-tertiary)]">미설치</span>}
          </div>
          {!status.ollama.installed && (
            <Button variant="ghost" size="sm" onClick={() => openUrl(INSTALL_URLS.ollama)}>
              <ExternalLink size={12} className="mr-1" /> 설치
            </Button>
          )}
          {status.ollama.installed && status.ollama.detail !== "실행 중" && (
            <Button variant="ghost" size="sm" onClick={() => invoke("open_url", { url: "x-terminal-emulator:" }).catch(() => {})}>
              ollama serve
            </Button>
          )}
        </div>
        {status.ollama.installed && status.ollama.detail !== "실행 중" && (
          <p className="text-xs text-[var(--text-tertiary)] mt-2 px-1">
            벡터 검색을 사용하려면 터미널에서 <code className="bg-[var(--bg-secondary)] px-1 rounded">ollama serve</code>를 실행하세요.
          </p>
        )}
      </Card>

      {/* Obsidian */}
      <Card>
        <h3 className="font-medium text-[var(--text-primary)] mb-3">Obsidian</h3>
        <div className="flex items-center justify-between p-3 rounded-lg bg-[var(--bg-primary)] border border-[var(--border)]">
          <div className="flex items-center gap-2">
            <StatusIcon ok={status.obsidian.installed} />
            <span className="text-sm font-medium text-[var(--text-primary)]">Obsidian</span>
            <span className="text-xs text-[var(--text-tertiary)]">
              {status.obsidian.installed ? "설치됨" : "미설치"}
            </span>
          </div>
          {!status.obsidian.installed && (
            <Button variant="ghost" size="sm" onClick={() => openUrl(INSTALL_URLS.obsidian)}>
              <ExternalLink size={12} className="mr-1" /> 설치
            </Button>
          )}
        </div>
      </Card>

      {/* Claude CLI 온보딩 */}
      <Card>
        <h3 className="font-medium text-[var(--text-primary)] mb-1">Claude CLI 온보딩</h3>
        <p className="text-xs text-[var(--text-tertiary)] mb-3">
          프로젝트에 Nexus MCP 서버, 권한 설정, 검색 가이드를 한 번에 설정합니다.
        </p>
        <div className="flex gap-2 mb-3">
          <input
            type="text"
            value={onboardPath}
            onChange={(e) => setOnboardPath(e.target.value)}
            placeholder="프로젝트 경로 (예: /Users/me/my-project)"
            className="flex-1 text-sm px-3 py-1.5 rounded-lg border border-[var(--border)] bg-[var(--bg-primary)] text-[var(--text-primary)] placeholder:text-[var(--text-tertiary)] focus:outline-none focus:border-[var(--accent)]"
          />
          <Button variant="ghost" size="sm" onClick={handlePickFolder} title="폴더 선택">
            <FolderOpen size={14} />
          </Button>
          <Button
            variant="secondary"
            size="sm"
            onClick={handleOnboard}
            disabled={onboarding || !onboardPath.trim()}
          >
            {onboarding ? <RefreshCw size={12} className="mr-1 animate-spin" /> : null}
            온보딩 시작
          </Button>
        </div>
        {onboardResults && (
          <div className="space-y-1">
            {onboardResults.map((step, i) => (
              <div key={i} className="flex items-center gap-2 text-sm p-2 rounded-lg bg-[var(--bg-primary)] border border-[var(--border)]">
                {step.status === "created" && <CheckCircle size={14} className="text-green-500 shrink-0" />}
                {step.status === "skipped" && <AlertCircle size={14} className="text-yellow-500 shrink-0" />}
                {step.status === "error" && <XCircle size={14} className="text-red-400 shrink-0" />}
                <span className="font-medium text-[var(--text-primary)]">{step.name}</span>
                <span className="text-[var(--text-tertiary)]">{step.message}</span>
              </div>
            ))}
            {onboardResults.every(s => s.status !== "error") && (
              <p className="text-xs text-[var(--text-tertiary)] pt-1 px-1">
                Claude Code 세션을 재시작하면 적용됩니다.
              </p>
            )}
          </div>
        )}
      </Card>

      <div className="flex justify-end">
        <Button variant="ghost" size="sm" onClick={load}>
          <RefreshCw size={12} className="mr-1" /> 새로고침
        </Button>
      </div>
    </div>
  );
}
