import { useState, useEffect, useCallback } from "react";
import { check, Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

interface UpdateCheckerProps {
  variant: "badge" | "settings";
}

export function UpdateChecker({ variant }: UpdateCheckerProps) {
  const [update, setUpdate] = useState<Update | null>(null);
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [progress, setProgress] = useState<string>("");
  const [error, setError] = useState<string>("");

  const checkForUpdate = useCallback(async () => {
    setChecking(true);
    setError("");
    try {
      const result = await check();
      setUpdate(result);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setChecking(false);
    }
  }, []);

  useEffect(() => {
    checkForUpdate();
  }, [checkForUpdate]);

  const handleInstall = async () => {
    if (!update) return;
    setInstalling(true);
    setProgress("다운로드 중...");
    try {
      let totalSize = 0;
      let downloaded = 0;
      await update.downloadAndInstall((event) => {
        if (event.event === "Started" && event.data.contentLength) {
          totalSize = event.data.contentLength;
          setProgress(`다운로드 중... 0%`);
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength || 0;
          const pct = totalSize > 0 ? Math.round((downloaded / totalSize) * 100) : 0;
          setProgress(`다운로드 중... ${pct}%`);
        } else if (event.event === "Finished") {
          setProgress("설치 중...");
        }
      });
      setProgress("재시작 중...");
      await relaunch();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setInstalling(false);
    }
  };

  if (variant === "badge") {
    if (!update?.available) return null;
    return (
      <button
        onClick={handleInstall}
        disabled={installing}
        className="px-2 py-0.5 rounded text-xs font-medium animate-pulse"
        style={{
          background: "var(--accent)",
          color: "var(--bg-primary)",
          opacity: installing ? 0.6 : 1,
        }}
      >
        {installing ? progress : `v${update.version} 업데이트`}
      </button>
    );
  }

  // settings variant
  return (
    <div
      className="p-4 rounded-lg"
      style={{ background: "var(--bg-secondary)", border: "1px solid var(--border)" }}
    >
      <div className="flex items-center justify-between mb-3">
        <h3 className="font-semibold" style={{ color: "var(--text-primary)" }}>
          앱 업데이트
        </h3>
        <span className="text-xs" style={{ color: "var(--text-secondary)" }}>
          현재 v{__APP_VERSION__}
        </span>
      </div>

      {error && (
        <p className="text-xs mb-2" style={{ color: "#ef4444" }}>
          오류: {error}
        </p>
      )}

      {update?.available ? (
        <div>
          <p className="text-sm mb-2" style={{ color: "var(--text-primary)" }}>
            새 버전 <strong>v{update.version}</strong> 사용 가능
          </p>
          <button
            onClick={handleInstall}
            disabled={installing}
            className="px-4 py-2 rounded text-sm font-medium"
            style={{
              background: installing ? "var(--border)" : "var(--accent)",
              color: "var(--bg-primary)",
            }}
          >
            {installing ? progress : "업데이트 설치"}
          </button>
        </div>
      ) : (
        <div className="flex items-center gap-2">
          <p className="text-sm" style={{ color: "var(--text-secondary)" }}>
            {checking ? "확인 중..." : "최신 버전입니다"}
          </p>
          {!checking && (
            <button
              onClick={checkForUpdate}
              className="px-3 py-1 rounded text-xs"
              style={{ background: "var(--border)", color: "var(--text-primary)" }}
            >
              다시 확인
            </button>
          )}
        </div>
      )}
    </div>
  );
}

declare const __APP_VERSION__: string;
