import { forwardRef, type InputHTMLAttributes } from "react";
import { cn } from "../../lib/utils";

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  sizing?: "sm" | "md";
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ sizing = "md", className, ...props }, ref) => (
    <input
      ref={ref}
      className={cn(
        "bg-[var(--bg-tertiary)] border border-[var(--border)] text-[var(--text-primary)]",
        "placeholder:text-[var(--text-tertiary)]",
        "focus:border-[var(--accent)] focus:ring-1 focus:ring-[var(--accent-muted)]",
        "outline-none transition-all duration-150",
        sizing === "sm" ? "px-2 py-1 text-xs rounded-md" : "px-3 py-2 text-sm rounded-lg",
        className,
      )}
      {...props}
    />
  ),
);
Input.displayName = "Input";
