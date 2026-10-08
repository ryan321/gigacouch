import type { ReactNode } from "react";

/** A labeled form field with an optional hint and its error. */
export function Field({ label, hint, error, children, className }: { label: string; hint?: string; error?: string; children: ReactNode; className?: string }) {
  return (
    <label className={className ? `field ${className}` : "field"}>
      <span className="label">{label}</span>
      {hint && <span className="hint">{hint}</span>}
      {children}
      {error && <span className="field-error">{error}</span>}
    </label>
  );
}
