import { cn } from "../../lib/utils";

type BadgeVariant = "success" | "warning" | "danger" | "info" | "muted";

interface BadgeProps {
  variant?: BadgeVariant;
  children: React.ReactNode;
  className?: string;
}

const variantStyles: Record<BadgeVariant, string> = {
  success: "bg-[var(--success)]/15 text-[var(--success)]",
  warning: "bg-[var(--warning)]/15 text-[var(--warning)]",
  danger: "bg-[var(--danger)]/15 text-[var(--danger)]",
  info: "bg-[var(--info)]/15 text-[var(--info)]",
  muted: "bg-[var(--bg-tertiary)] text-[var(--text-tertiary)]",
};

export function Badge({ variant = "info", className, children }: BadgeProps) {
  return (
    <span className={cn("inline-flex items-center px-2 py-0.5 rounded-md text-xs font-medium", variantStyles[variant], className)}>
      {children}
    </span>
  );
}
