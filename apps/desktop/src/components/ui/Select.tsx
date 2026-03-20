import { forwardRef, type SelectHTMLAttributes } from "react";
import { cn } from "../../lib/utils";

export const Select = forwardRef<HTMLSelectElement, SelectHTMLAttributes<HTMLSelectElement>>(
  ({ className, ...props }, ref) => (
    <select
      ref={ref}
      className={cn(
        "bg-[var(--bg-tertiary)] border border-[var(--border)] text-[var(--text-primary)]",
        "px-2 py-1 text-xs rounded-md outline-none",
        "focus:border-[var(--accent)] transition-all duration-150",
        className,
      )}
      {...props}
    />
  ),
);
Select.displayName = "Select";
