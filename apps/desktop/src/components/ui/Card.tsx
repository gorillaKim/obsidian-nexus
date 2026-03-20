import type { HTMLAttributes } from "react";
import { cn } from "../../lib/utils";

interface CardProps extends HTMLAttributes<HTMLDivElement> {
  interactive?: boolean;
}

export function Card({ interactive, className, ...props }: CardProps) {
  return (
    <div
      className={cn(
        "rounded-lg border border-[var(--border)] bg-[var(--bg-secondary)] p-4",
        "transition-colors duration-150",
        interactive && "hover:border-[var(--border-hover)] cursor-pointer",
        className,
      )}
      {...props}
    />
  );
}
