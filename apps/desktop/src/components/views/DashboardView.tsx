import { useState } from "react";
import { LayoutDashboard, Eye, Link2, ChevronDown, ExternalLink } from "lucide-react";
import { Card } from "../ui/Card";
import { EmptyState } from "../ui/EmptyState";
import type { PopularDoc, TopProject, AttentionDoc, AttentionReason } from "../../types";

interface DashboardViewProps {
  totalProjects: number;
  totalDocs: number;
  totalChunks: number;
  popularAll: PopularDoc[];
  topProjects: TopProject[];
  popularByProject: Map<string, PopularDoc[]>;
  allProjects: { id: string; name: string }[];
  attentionDocs: AttentionDoc[];
  loading: boolean;
  onOpenFile: (projectId: string, filePath: string) => void;
  onViewDocument: (projectId: string, filePath: string) => void;
  onSelectExtraProject: (projectId: string) => void;
}

const RANK_COLORS = ["text-yellow-500", "text-gray-400", "text-orange-400"];

function RankBadge({ rank }: { rank: number }) {
  const color = rank <= 3 ? RANK_COLORS[rank - 1] : "text-[var(--text-tertiary)]";
  return (
    <span className={`w-6 text-center text-xs font-bold tabular-nums ${color}`}>
      {rank}
    </span>
  );
}

function SkeletonRow() {
  return (
    <div className="flex items-center gap-3 py-2 px-3 animate-pulse">
      <div className="w-6 h-4 bg-[var(--bg-tertiary)] rounded" />
      <div className="flex-1 h-4 bg-[var(--bg-tertiary)] rounded" />
      <div className="w-20 h-4 bg-[var(--bg-tertiary)] rounded" />
      <div className="w-12 h-4 bg-[var(--bg-tertiary)] rounded" />
    </div>
  );
}

function RankingList({
  docs,
  loading,
  onOpenFile,
  onViewDocument,
}: {
  docs: PopularDoc[];
  loading: boolean;
  onOpenFile: (projectId: string, filePath: string) => void;
  onViewDocument: (projectId: string, filePath: string) => void;
}) {
  if (loading) {
    return (
      <div className="divide-y divide-[var(--border)]">
        {Array.from({ length: 5 }).map((_, i) => <SkeletonRow key={i} />)}
      </div>
    );
  }

  if (docs.length === 0) {
    return (
      <div className="py-8 text-center text-xs text-[var(--text-tertiary)]">
        아직 조회된 문서가 없습니다. 검색 후 랭킹이 표시됩니다.
      </div>
    );
  }

  return (
    <div className="divide-y divide-[var(--border)]">
      {docs.map((doc, i) => (
        <div
          key={doc.id}
          className="flex items-center gap-3 py-2 px-3 hover:bg-[var(--bg-secondary)] transition-colors"
        >
          <RankBadge rank={i + 1} />
          <button
            onClick={() => onViewDocument(doc.project_id, doc.file_path)}
            className="flex-1 text-sm text-[var(--text-primary)] truncate text-left hover:underline"
          >
            {doc.title}
          </button>
          <span className="text-xs text-[var(--text-tertiary)] truncate max-w-[100px]">{doc.project_name}</span>
          <span className="flex items-center gap-1 text-xs text-[var(--text-tertiary)] min-w-[40px]">
            <Eye size={11} />
            {doc.view_count}
          </span>
          <span className="flex items-center gap-1 text-xs text-[var(--text-tertiary)] min-w-[40px]">
            <Link2 size={11} />
            {doc.backlink_count}
          </span>
          <button
            onClick={() => onOpenFile(doc.project_id, doc.file_path)}
            className="text-[var(--text-tertiary)] hover:text-[var(--text-primary)] transition-colors"
            title="Obsidian에서 열기"
          >
            <ExternalLink size={12} />
          </button>
        </div>
      ))}
    </div>
  );
}

const REASON_CONFIG: Record<AttentionReason, { label: string; color: string }> = {
  never_viewed: { label: "미열람", color: "text-blue-400 bg-blue-400/10" },
  orphan: { label: "고아", color: "text-orange-400 bg-orange-400/10" },
  stale: { label: "오래됨", color: "text-yellow-500 bg-yellow-500/10" },
};

function ReasonBadge({ reason }: { reason: AttentionReason }) {
  const cfg = REASON_CONFIG[reason];
  return (
    <span className={`shrink-0 text-[10px] font-medium px-1.5 py-0.5 rounded ${cfg.color}`}>
      {cfg.label}
    </span>
  );
}

function AttentionList({
  docs,
  loading,
  onOpenFile,
  onViewDocument,
}: {
  docs: AttentionDoc[];
  loading: boolean;
  onOpenFile: (projectId: string, filePath: string) => void;
  onViewDocument: (projectId: string, filePath: string) => void;
}) {
  if (loading) {
    return (
      <div className="divide-y divide-[var(--border)]">
        {Array.from({ length: 4 }).map((_, i) => <SkeletonRow key={i} />)}
      </div>
    );
  }

  if (docs.length === 0) {
    return (
      <div className="py-6 text-center text-xs text-[var(--text-tertiary)]">
        관심이 필요한 문서가 없습니다. 문서가 잘 관리되고 있습니다!
      </div>
    );
  }

  return (
    <div className="divide-y divide-[var(--border)]">
      {docs.map((doc) => (
        <div
          key={doc.id}
          className="flex items-center gap-3 py-2 px-3 hover:bg-[var(--bg-secondary)] transition-colors"
        >
          <ReasonBadge reason={doc.reason as AttentionReason} />
          <button
            onClick={() => onViewDocument(doc.project_id, doc.file_path)}
            className="flex-1 text-sm text-[var(--text-primary)] truncate text-left hover:underline"
          >
            {doc.title}
          </button>
          <span className="text-xs text-[var(--text-tertiary)] truncate max-w-[100px]">{doc.project_name}</span>
          <span className="flex items-center gap-1 text-xs text-[var(--text-tertiary)] min-w-[40px]">
            <Eye size={11} />
            {doc.view_count}
          </span>
          <span className="flex items-center gap-1 text-xs text-[var(--text-tertiary)] min-w-[40px]">
            <Link2 size={11} />
            {doc.backlink_count}
          </span>
          <button
            onClick={() => onOpenFile(doc.project_id, doc.file_path)}
            className="text-[var(--text-tertiary)] hover:text-[var(--text-primary)] transition-colors"
            title="Obsidian에서 열기"
          >
            <ExternalLink size={12} />
          </button>
        </div>
      ))}
    </div>
  );
}

export function DashboardView({
  totalProjects,
  totalDocs,
  totalChunks,
  popularAll,
  topProjects,
  popularByProject,
  allProjects,
  attentionDocs,
  loading,
  onOpenFile,
  onViewDocument,
  onSelectExtraProject,
}: DashboardViewProps) {
  // 탭: "all" | project_id
  const [activeTab, setActiveTab] = useState<string>("all");
  const [dropdownOpen, setDropdownOpen] = useState(false);

  // 탭에 표시할 프로젝트 (top 2 고정)
  const pinnedProjects = topProjects.slice(0, 2);
  // 나머지 프로젝트 (더보기 드롭다운용)
  const extraProjects = allProjects.filter(
    (p) => !pinnedProjects.some((tp) => tp.id === p.id)
  );

  const currentDocs =
    activeTab === "all"
      ? popularAll
      : (popularByProject.get(activeTab) ?? []);

  const stats = [
    { label: "등록된 프로젝트", value: totalProjects },
    { label: "인덱싱된 문서", value: totalDocs },
    { label: "총 청크", value: totalChunks },
  ];

  return (
    <div className="p-6 max-w-4xl mx-auto">
      {/* Stats */}
      <div className="grid grid-cols-3 gap-4 mb-8">
        {stats.map((stat) => (
          <Card key={stat.label} className="text-center py-6">
            <div className="text-3xl font-bold text-[var(--accent)] mb-1">{stat.value}</div>
            <div className="text-xs text-[var(--text-tertiary)]">{stat.label}</div>
          </Card>
        ))}
      </div>

      {/* Popular docs ranking */}
      <h2 className="text-xs font-medium text-[var(--text-tertiary)] uppercase tracking-wider mb-3">
        인기 문서 랭킹
      </h2>
      <Card className="overflow-visible">
        {/* Tab bar */}
        <div className="flex items-center border-b border-[var(--border)] px-1 relative">
          {/* 전체 탭 */}
          <TabButton
            active={activeTab === "all"}
            onClick={() => setActiveTab("all")}
          >
            전체
          </TabButton>

          {/* 상위 2개 프로젝트 탭 */}
          {pinnedProjects.map((p) => (
            <TabButton
              key={p.id}
              active={activeTab === p.id}
              onClick={() => setActiveTab(p.id)}
            >
              {p.name}
            </TabButton>
          ))}

          {/* 더보기 드롭다운 (3개 이상일 때) */}
          {extraProjects.length > 0 && (
            <div className="relative ml-1">
              <button
                onClick={() => setDropdownOpen((v) => !v)}
                className="flex items-center gap-1 px-2 py-2 text-xs text-[var(--text-tertiary)] hover:text-[var(--text-primary)] transition-colors"
              >
                더보기 <ChevronDown size={11} />
              </button>
              {dropdownOpen && (
                <div className="absolute top-full left-0 mt-1 bg-[var(--bg-primary)] border border-[var(--border)] rounded shadow-lg z-10 min-w-[140px]">
                  {extraProjects.map((p) => (
                    <button
                      key={p.id}
                      onClick={() => {
                        setActiveTab(p.id);
                        setDropdownOpen(false);
                        onSelectExtraProject(p.id);
                      }}
                      className="w-full text-left px-3 py-2 text-xs text-[var(--text-primary)] hover:bg-[var(--bg-secondary)] transition-colors"
                    >
                      {p.name}
                    </button>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>

        {/* Ranking list */}
        <RankingList docs={currentDocs} loading={loading} onOpenFile={onOpenFile} onViewDocument={onViewDocument} />
      </Card>

      {/* Attention-needed documents */}
      <h2 className="text-xs font-medium text-[var(--text-tertiary)] uppercase tracking-wider mb-3 mt-8">
        관심 필요 문서
      </h2>
      <Card>
        <AttentionList
          docs={attentionDocs}
          loading={loading}
          onOpenFile={onOpenFile}
          onViewDocument={onViewDocument}
        />
      </Card>

      {/* Empty state when no projects */}
      {totalProjects === 0 && !loading && (
        <div className="mt-6">
          <EmptyState
            icon={<LayoutDashboard size={40} />}
            title="등록된 프로젝트가 없습니다"
            description="프로젝트 탭에서 볼트 폴더를 추가하세요"
          />
        </div>
      )}
    </div>
  );
}

function TabButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={`px-3 py-2 text-xs font-medium border-b-2 transition-colors ${
        active
          ? "border-[var(--accent)] text-[var(--accent)]"
          : "border-transparent text-[var(--text-tertiary)] hover:text-[var(--text-primary)]"
      }`}
    >
      {children}
    </button>
  );
}
