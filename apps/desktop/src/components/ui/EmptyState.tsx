import type { ReactNode } from "react";
import { cn } from "../../lib/utils";

interface EmptyStateProps {
  icon?: ReactNode;
  title: string;
  description?: string;
  action?: ReactNode;
  className?: string;
}

export function EmptyState({ icon, title, description, action, className }: EmptyStateProps) {
  return (
    <div className={cn("flex flex-col items-center justify-center py-16 text-center", className)}>
      {icon && <div className="mb-4 text-[var(--text-tertiary)]">{icon}</div>}
      <p className="text-base font-medium text-[var(--text-secondary)] mb-1">{title}</p>
      {description && <p className="text-sm text-[var(--text-tertiary)] mb-4">{description}</p>}
      {action}
    </div>
  );
}
