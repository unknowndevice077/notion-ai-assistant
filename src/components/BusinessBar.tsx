import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { Business } from "../lib/types";

interface BusinessBarProps {
  selected: Business | null;
  onSelect: (business: Business | null) => void;
  busy: boolean;
}

/**
 * Lists every business/child page the app has created in Notion so far.
 * Picking one just SELECTS it — the actual instruction is typed into the
 * main CommandBar below, same as any other prompt. While a business is
 * selected, that prompt is scoped to ONLY that business's existing
 * page/databases — it never creates a new page, just edits/adds to the
 * one selected.
 */
export default function BusinessBar({ selected, onSelect, busy }: BusinessBarProps) {
  const [open, setOpen] = useState(false);
  const [businesses, setBusinesses] = useState<Business[]>([]);

  const refresh = () => api.listBusinesses().then(setBusinesses).catch(() => setBusinesses([]));

  useEffect(() => {
    refresh();
  }, []);

  useEffect(() => {
    if (open) refresh();
  }, [open]);

  const pick = (business: Business | null) => {
    onSelect(business);
    setOpen(false);
  };

  return (
    <div className="relative">
      <button
        onClick={() => setOpen((v) => !v)}
        disabled={busy}
        className={`flex items-center gap-1.5 rounded-sm border px-2.5 py-1.5 text-xs font-medium transition-colors disabled:opacity-40 ${
          selected
            ? "border-accent bg-accent-muted text-ink-100"
            : "border-border bg-surface-2 text-ink-70 hover:border-border-strong hover:text-ink-100"
        }`}
      >
        <span className="max-w-[10rem] truncate">{selected ? selected.name : "Edit Business Page"}</span>
        <span className={`shrink-0 transition-transform ${open ? "rotate-180" : ""}`}>▲</span>
      </button>

      {open && (
        <div className="absolute bottom-full left-0 z-10 mb-2 w-64 max-w-[calc(100vw-1.5rem)] max-h-[70vh] overflow-y-auto rounded-lg border border-border bg-surface-1 p-2 shadow-lg">
          {businesses.length === 0 ? (
            <p className="p-1.5 text-xs text-ink-40">
              No businesses yet — run the Business Preset once to create one, then it'll show up here.
            </p>
          ) : (
            <div className="flex flex-col gap-0.5">
              {selected && (
                <button
                  onClick={() => pick(null)}
                  className="rounded-sm px-2.5 py-1.5 text-left text-xs text-ink-40 hover:bg-surface-2 hover:text-ink-100"
                >
                  ✕ Clear selection (general mode)
                </button>
              )}
              {businesses.map((b) => (
                <button
                  key={b.id}
                  onClick={() => pick(b)}
                  className={`rounded-sm px-2.5 py-1.5 text-left text-sm transition-colors ${
                    selected?.id === b.id ? "bg-accent-muted text-ink-100" : "text-ink-70 hover:bg-surface-2 hover:text-ink-100"
                  }`}
                >
                  <div className="truncate">{b.name}</div>
                  {b.context && <div className="truncate text-[10px] text-ink-40">{b.context}</div>}
                </button>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}