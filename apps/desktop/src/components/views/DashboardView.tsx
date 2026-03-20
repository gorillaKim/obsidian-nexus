import { LayoutDashboard, RefreshCw } from "lucide-react";
import { Card } from "../ui/Card";
import { Button } from "../ui/Button";
import { Badge } from "../ui/Badge";
import { EmptyState } from "../ui/EmptyState";
import type { Project, ProjectInfo } from "../../types";

interface DashboardViewProps {
  projects: Project[];
  projectInfos: Map<string, ProjectInfo>;
  totalDocs: number;
  indexing: Set<string>;
  onIndex: (projectId: string) => void;
}

export function DashboardView({ projects, projectInfos, totalDocs, indexing, onIndex }: DashboardViewProps) {
  return (
    <div className="p-6 max-w-4xl mx-auto">
      {/* Stats */}
      <div className="grid grid-cols-2 gap-4 mb-8">
        {[
          { label: "등록된 프로젝트", value: projects.length },
          { label: "인덱싱된 문서", value: totalDocs },
        ].map((stat) => (
          <Card key={stat.label} className="text-center py-6">
            <div className="text-3xl font-bold text-[var(--accent)] mb-1">{stat.value}</div>
            <div className="text-xs text-[var(--text-tertiary)]">{stat.label}</div>
          </Card>
        ))}
      </div>

      {/* Project status */}
      <h2 className="text-xs font-medium text-[var(--text-tertiary)] uppercase tracking-wider mb-3">프로젝트 현황</h2>
      <div className="space-y-2">
        {projects.map((p) => {
          const info = projectInfos.get(p.id);
          return (
            <Card key={p.id} className="flex items-center justify-between py-3 px-4">
              <div className="flex items-center gap-3">
                <span className="font-medium text-sm text-[var(--text-primary)]">{p.name}</span>
                {info && (
                  <span className="text-xs text-[var(--text-tertiary)]">{info.stats.doc_count}개 문서</span>
                )}
                {info && info.stats.pending_count > 0 && (
                  <Badge variant="warning">{info.stats.pending_count}개 대기 중</Badge>
                )}
              </div>
              <div className="flex items-center gap-3">
                <span className="text-xs text-[var(--text-tertiary)]">
                  {p.last_indexed_at ? `${new Date(p.last_indexed_at).toLocaleDateString("ko-KR")} 인덱싱됨` : "미인덱싱"}
                </span>
                <Button
                  variant="primary"
                  size="sm"
                  onClick={() => onIndex(p.id)}
                  disabled={indexing.has(p.id)}
                >
                  {indexing.has(p.id) ? (
                    <><RefreshCw size={12} className="animate-spin mr-1" /> 인덱싱 중...</>
                  ) : "인덱싱"}
                </Button>
              </div>
            </Card>
          );
        })}
        {projects.length === 0 && (
          <EmptyState
            icon={<LayoutDashboard size={40} />}
            title="등록된 프로젝트가 없습니다"
            description="프로젝트 탭에서 볼트 폴더를 추가하세요"
          />
        )}
      </div>
    </div>
  );
}
