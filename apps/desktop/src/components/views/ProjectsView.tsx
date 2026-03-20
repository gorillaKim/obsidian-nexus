import { FolderOpen, RefreshCw, Trash2, FolderSync } from "lucide-react";
import { Card } from "../ui/Card";
import { Button } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";
import type { Project, ProjectInfo } from "../../types";

interface ProjectsViewProps {
  projects: Project[];
  projectInfos: Map<string, ProjectInfo>;
  indexing: Set<string>;
  adding: boolean;
  syncing: Set<string>;
  onIndex: (projectId: string) => void;
  onAddVault: () => void;
  onSync: (projectId: string) => void;
  onRemove: (projectId: string) => void;
}

export function ProjectsView({
  projects, projectInfos, indexing, adding, syncing,
  onIndex, onAddVault, onSync, onRemove,
}: ProjectsViewProps) {
  return (
    <div className="p-6 max-w-4xl mx-auto space-y-3">
      <button
        onClick={onAddVault}
        disabled={adding}
        className="w-full px-4 py-4 rounded-lg text-sm font-medium border-2 border-dashed border-[var(--accent)] text-[var(--accent)] hover:bg-[var(--accent-muted)] transition-colors duration-150 cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
      >
        {adding ? (
          <span className="flex items-center justify-center gap-2">
            <RefreshCw size={14} className="animate-spin" /> 볼트 추가 및 인덱싱 중...
          </span>
        ) : "+ 볼트 폴더 추가"}
      </button>

      {projects.map((p) => {
        const info = projectInfos.get(p.id);
        return (
          <Card key={p.id}>
            <div className="flex items-center justify-between mb-2">
              <h3 className="font-medium text-[var(--text-primary)]">{p.name}</h3>
              <div className="flex gap-2">
                <Button variant="primary" size="sm" onClick={() => onIndex(p.id)} disabled={indexing.has(p.id)}>
                  {indexing.has(p.id) ? <><RefreshCw size={12} className="animate-spin mr-1" /> 인덱싱 중...</> : "인덱싱"}
                </Button>
                <Button variant="secondary" size="sm" onClick={() => onSync(p.id)} disabled={syncing.has(p.id)}>
                  {syncing.has(p.id) ? <><FolderSync size={12} className="animate-spin mr-1" /> 동기화 중...</> : "동기화"}
                </Button>
                <Button variant="danger" size="sm" onClick={() => onRemove(p.id)}>
                  <Trash2 size={12} className="mr-1" /> 삭제
                </Button>
              </div>
            </div>
            <p className="text-xs text-[var(--text-tertiary)] mb-1">{p.path}</p>
            <p className="text-xs text-[var(--text-tertiary)]">
              마지막 인덱싱: {p.last_indexed_at ? new Date(p.last_indexed_at).toLocaleString("ko-KR") : "없음"}
            </p>
            {info && (
              <div className="flex gap-4 mt-2">
                <span className="text-xs"><span className="text-[var(--accent)]">{info.stats.doc_count}</span> 문서</span>
                {info.stats.pending_count > 0 && (
                  <span className="text-xs text-[var(--warning)]">{info.stats.pending_count}개 대기 중</span>
                )}
              </div>
            )}
          </Card>
        );
      })}

      {projects.length === 0 && (
        <EmptyState
          icon={<FolderOpen size={40} />}
          title="등록된 프로젝트가 없습니다"
          description="위의 '볼트 폴더 추가' 버튼을 눌러 Obsidian 볼트를 등록하세요"
        />
      )}
    </div>
  );
}
