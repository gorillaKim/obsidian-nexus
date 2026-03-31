interface FrontmatterCardProps {
  data: Record<string, unknown>;
  onTagClick: (tag: string) => void;
}

export function FrontmatterCard({ data, onTagClick }: FrontmatterCardProps) {
  const entries = Object.entries(data).filter(([k]) => k !== "title");
  if (entries.length === 0) return null;

  return (
    <div className="mb-4 rounded-lg border border-[var(--border)] bg-[var(--bg-secondary)] px-4 py-3 text-xs space-y-2">
      {entries.map(([key, value]) => (
        <div key={key} className="flex items-start gap-2">
          <span className="text-[var(--text-tertiary)] w-20 shrink-0 capitalize">{key}</span>
          {key === "tags" && Array.isArray(value) ? (
            <div className="flex flex-wrap gap-1">
              {value.map((tag) => (
                <button
                  key={String(tag)}
                  onClick={() => onTagClick(String(tag))}
                  title={`#${tag} 태그로 검색`}
                  className="rounded-full bg-[var(--accent-muted)] px-2 py-0.5 text-[var(--accent)] hover:bg-[var(--accent)]/30 transition-colors"
                >
                  #{tag}
                </button>
              ))}
            </div>
          ) : typeof value === "string" && /^\d{4}-\d{2}-\d{2}/.test(value) ? (
            <span className="text-[var(--text-secondary)]">
              {new Date(value).toLocaleDateString("ko-KR", {
                year: "numeric",
                month: "long",
                day: "numeric",
              })}
            </span>
          ) : (
            <span className="text-[var(--text-secondary)]">{String(value)}</span>
          )}
        </div>
      ))}
    </div>
  );
}
