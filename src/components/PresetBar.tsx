import { useState } from "react";
import type { Preset } from "../lib/types";

interface PresetBarProps {
  presets: Preset[];
  onRun: (preset: Preset, fieldValues: Record<string, string>) => void;
  busy: boolean;
}

const AMOUNT_FIELD = "Amount of content generation";
const START_DATE_FIELD = "Start Date";

function todayIso(): string {
  const d = new Date();
  const tzOffsetMs = d.getTimezoneOffset() * 60_000;
  return new Date(d.getTime() - tzOffsetMs).toISOString().slice(0, 10);
}

export default function PresetBar({ presets, onRun, busy }: PresetBarProps) {
  const [open, setOpen] = useState(false);
  const [fieldValues, setFieldValues] = useState<Record<string, string>>({ [AMOUNT_FIELD]: "10", [START_DATE_FIELD]: todayIso() });

  // There's one preset now ("business"), so open the form straight away.
  const preset = presets[0];

  const set = (field: string, value: string) => setFieldValues((prev) => ({ ...prev, [field]: value }));

  const run = () => {
    if (!preset) return;
    onRun(preset, fieldValues);
    setOpen(false);
  };

  if (!preset) return null;

  return (
    <div className="relative">
      <button
        onClick={() => setOpen((v) => !v)}
        disabled={busy}
        className="flex items-center gap-1.5 rounded-sm border border-border bg-surface-2 px-2.5 py-1.5 text-xs font-medium text-ink-70 transition-colors hover:border-border-strong hover:text-ink-100 disabled:opacity-40"
      >
        <span>Business Preset</span>
        <span className={`transition-transform ${open ? "rotate-180" : ""}`}>▲</span>
      </button>

      {open && (
        <div className="absolute bottom-full left-0 z-10 mb-2 w-72 max-w-[calc(100vw-1.5rem)] max-h-[70vh] overflow-y-auto rounded-lg border border-border bg-surface-1 p-3 shadow-lg">
          <div className="flex flex-col gap-2.5">
            <div className="flex flex-col gap-1">
              <span className="text-xs text-ink-70">Business Name:</span>
              <input
                value={fieldValues["Business Name"] ?? ""}
                onChange={(e) => set("Business Name", e.target.value)}
                placeholder="Acme Studio"
                className="rounded-sm border border-border bg-surface-2 px-2.5 py-1.5 text-sm text-ink-100 outline-none focus:border-accent"
              />
            </div>

            <div className="flex flex-col gap-1">
              <span className="text-xs text-ink-70">What's the business about? (optional)</span>
              <textarea
                value={fieldValues["Business Context"] ?? ""}
                onChange={(e) => set("Business Context", e.target.value)}
                placeholder="e.g. a specialty coffee roastery focused on single-origin beans and pour-over brewing"
                rows={2}
                className="resize-none rounded-sm border border-border bg-surface-2 px-2.5 py-1.5 text-sm text-ink-100 outline-none focus:border-accent"
              />
              <span className="text-[10px] text-ink-40">
                Helps the agent write more relevant content, and picks a more fitting cover photo for the Notion page.
              </span>
            </div>

            <div className="flex flex-col gap-1">
              <span className="text-xs text-ink-70">Start date:</span>
              <input
                type="date"
                value={fieldValues[START_DATE_FIELD] ?? todayIso()}
                onChange={(e) => set(START_DATE_FIELD, e.target.value)}
                className="rounded-sm border border-border bg-surface-2 px-2.5 py-1.5 text-sm text-ink-100 outline-none focus:border-accent"
              />
              <span className="text-[10px] text-ink-40">First day of the calendar. Each following day gets its own dated batch.</span>
            </div>

            <div className="flex flex-col gap-1">
              <span className="text-xs text-ink-70">Days of content:</span>
              <div className="flex items-center gap-2">
                <input
                  type="number"
                  min={1}
                  value={fieldValues[AMOUNT_FIELD] ?? ""}
                  onChange={(e) => set(AMOUNT_FIELD, e.target.value)}
                  placeholder="10"
                  className="w-20 rounded-sm border border-border bg-surface-2 px-2.5 py-1.5 text-sm text-ink-100 outline-none focus:border-accent"
                />
                <span className="text-xs text-ink-40">days</span>
              </div>
              <span className="text-[10px] text-ink-40">
                The calendar gets exactly the counts below, generated fresh for each day.
              </span>
            </div>

            {(["Headlines", "Quotes", "Tips"] as const).map((field) => (
              <div key={field} className="flex flex-col gap-1">
                <span className="text-xs text-ink-70">{field} per day:</span>
                <input
                  value={fieldValues[field] ?? ""}
                  onChange={(e) => set(field, e.target.value)}
                  placeholder="e.g. 5"
                  className="rounded-sm border border-border bg-surface-2 px-2.5 py-1.5 text-sm text-ink-100 outline-none focus:border-accent"
                />
              </div>
            ))}

            <button
              onClick={run}
              disabled={busy}
              className="mt-1 rounded-sm bg-accent px-3 py-1.5 text-sm font-medium text-white hover:bg-accent-hover disabled:opacity-40"
            >
              {busy ? "Generating…" : "Run"}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}