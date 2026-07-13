import { useEffect, useState, useRef } from "react";
import { api } from "../lib/api";
import type { Business, ContentBatch, Preset } from "../lib/types";
import CommandBar from "./CommandBar";
import PresetBar from "./PresetBar";
import BusinessBar from "./BusinessBar";

const TYPE_LABEL: Record<string, string> = {
  headline: "Headline",
  subline: "Subline",
  quote: "Quote",
  tip: "Tip",
  calendar: "Calendar Entry",
};

const TYPE_LABEL_PLURAL: Record<string, string> = {
  headline: "headlines",
  subline: "sublines",
  quote: "quotes",
  tip: "tips",
  calendar: "calendar entries",
};

// One accent color per content type — mirrors the select-option colors
// used in the actual Notion Content Calendar, so the app visually matches
// what you'll see there.
const TYPE_ACCENT: Record<string, string> = {
  headline: "border-l-sky-500 text-sky-400",
  subline: "border-l-violet-500 text-violet-400",
  quote: "border-l-pink-500 text-pink-400",
  tip: "border-l-amber-500 text-amber-400",
  calendar: "border-l-teal-500 text-teal-400",
};

const BUSY_STAGES = ["Reading your request…", "Thinking it through…", "Drafting content…", "Pushing to Notion…"];
const STAGE_ADVANCE_MS = [1100, 2400, 2600];

export default function GenerateTab() {
  const [presets, setPresets] = useState<Preset[]>([]);
  const [batch, setBatch] = useState<ContentBatch | null>(null);
  const [busy, setBusy] = useState(false);
  const [busyStageIdx, setBusyStageIdx] = useState(0);
  const [busyElapsed, setBusyElapsed] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [decided, setDecided] = useState(false); // true once Accept/Decline is resolved
  const [discarding, setDiscarding] = useState(false);
  const [expandedIdx, setExpandedIdx] = useState<number | null>(null);
  const [scopedBusiness, setScopedBusiness] = useState<Business | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    api.getPresets().then(setPresets).catch(() => setPresets([]));
  }, []);

  useEffect(() => {
    if (scrollRef.current) scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
  }, [batch, error]);

  useEffect(() => {
    if (!busy) {
      setBusyStageIdx(0);
      setBusyElapsed(0);
      return;
    }
    setBusyStageIdx(0);
    setBusyElapsed(0);
    const timeouts = STAGE_ADVANCE_MS.map((ms, i) => {
      const at = STAGE_ADVANCE_MS.slice(0, i + 1).reduce((a, b) => a + b, 0);
      return setTimeout(() => setBusyStageIdx(i + 1), at);
    });
    const tick = setInterval(() => setBusyElapsed((s) => s + 1), 1000);
    return () => {
      timeouts.forEach(clearTimeout);
      clearInterval(tick);
    };
  }, [busy]);

  const busyLabel =
    busyStageIdx === BUSY_STAGES.length - 1 && busyElapsed > 3
      ? `${BUSY_STAGES[busyStageIdx]} (${busyElapsed}s)`
      : BUSY_STAGES[busyStageIdx];

  // Wipes the current result off the screen — a fresh generate/preset run
  // always REPLACES this state already (setBatch(result) overwrites, it
  // never appends), but there was no way to just clear it and go back to
  // a blank canvas without kicking off a new request. This is that.
  const clearResults = () => {
    setBatch(null);
    setError(null);
    setDecided(false);
    setExpandedIdx(null);
  };

  const runCommand = async (command: string) => {
    setBusy(true);
    setError(null);
    setDecided(false);
    setExpandedIdx(null);
    try {
      // A business is selected in the "Edit Business Page" dropdown — the
      // prompt is scoped to just that page instead of the general command.
      const result = scopedBusiness ? await api.editBusiness(scopedBusiness.id, command) : await api.generateContent(command);
      setBatch(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const runPreset = async (preset: Preset, fieldValues: Record<string, string>) => {
    setBusy(true);
    setError(null);
    setDecided(false);
    setExpandedIdx(null);
    try {
      const result = await api.runPreset(preset.id, fieldValues);
      setBatch(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  // Content is already live in Notion by the time this shows — Accept is
  // just closing out the decision, nothing to send.
  const acceptPush = () => setDecided(true);

  const declinePush = async () => {
    if (!batch) return;
    setDiscarding(true);
    setError(null);
    try {
      const res = await api.discardNotionPush(batch.id);
      if (res.ok) setDecided(true);
      else setError(res.message);
    } catch (err) {
      setError(String(err));
    } finally {
      setDiscarding(false);
    }
  };

  const isNotion404 = error?.includes("404") || error?.includes("object_not_found") || batch?.pushError?.includes("404");
  const wasPushed = batch?.status === "pushed";

  const typeCounts = batch
    ? batch.items.reduce<Record<string, number>>((acc, item) => {
        acc[item.type] = (acc[item.type] ?? 0) + 1;
        return acc;
      }, {})
    : {};

  return (
    <div className="flex h-full flex-col bg-surface-0 min-h-0">
      <div ref={scrollRef} className="flex-1 overflow-y-auto p-4 min-h-0 space-y-3">
        {busy && (
          <div className="flex items-center gap-2.5 rounded-xl border border-accent/40 bg-accent-muted/30 px-3.5 py-3 shadow-sm">
            <span className="relative flex h-2 w-2 shrink-0">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-accent opacity-60" />
              <span className="relative inline-flex h-2 w-2 rounded-full bg-accent" />
            </span>
            <span className="font-mono text-xs uppercase tracking-wider text-ink-100">{busyLabel}</span>
          </div>
        )}

        {error && !isNotion404 && (
          <div className="rounded-xl border border-red-900 bg-red-950/40 px-3.5 py-3 text-xs text-red-300 font-mono shadow-sm">
            <span className="font-bold uppercase">[ Error ]</span> {error}
          </div>
        )}

        {batch && (
          <div className="flex flex-col gap-3 rounded-xl border border-border bg-surface-1 p-4 shadow-sm">
            <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border pb-3 shrink-0">
              <div className="flex items-center gap-2">
                <span className="flex h-6 w-6 items-center justify-center rounded-full bg-accent-muted text-[11px] font-bold text-accent font-mono">
                  {batch.items.length}
                </span>
                <span className="text-[11px] font-bold uppercase tracking-wider text-ink-70 font-mono">
                  items {wasPushed ? "— live in Notion" : "— not pushed"}
                </span>
              </div>

              <div className="flex shrink-0 items-center gap-2">
                {wasPushed && !decided && (
                  <>
                    <button
                      onClick={acceptPush}
                      className="rounded-md bg-emerald-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-emerald-500 transition-colors"
                    >
                      Accept
                    </button>
                    <button
                      onClick={declinePush}
                      disabled={discarding}
                      className="rounded-md border border-red-900 bg-red-950/40 px-3 py-1.5 text-xs font-medium text-red-300 hover:bg-red-950/60 disabled:opacity-40 transition-colors"
                    >
                      {discarding ? "Removing…" : "Decline"}
                    </button>
                  </>
                )}
                {wasPushed && decided && <span className="text-xs font-medium text-emerald-400">Accepted ✓</span>}
                <button
                  onClick={clearResults}
                  title="Clear this result"
                  className="rounded-md border border-border bg-surface-2 px-2.5 py-1.5 text-xs font-medium text-ink-40 transition-colors hover:border-border-strong hover:text-ink-100"
                >
                  Clear
                </button>
              </div>
            </div>

            {/* Real reason the push failed, straight from the backend —
                no more guessing "Notion unavailable" when it's actually
                connected and something else went wrong. */}
            {!wasPushed && batch.pushError && !isNotion404 && (
              <div className="rounded-md border border-red-900 bg-red-950/30 px-3 py-2 text-xs text-red-300 font-mono">
                {batch.pushError}
              </div>
            )}

            {!wasPushed && batch.pushError && isNotion404 && (
              <div className="rounded-lg border border-red-900 bg-red-950/30 p-4 text-xs text-red-300 space-y-2">
                <div className="font-bold uppercase tracking-wider flex items-center gap-1.5">
                  <span>⚠️</span> Notion Workspace Link Broken (404)
                </div>
                <p className="leading-relaxed">{batch.pushError}</p>
                <div className="bg-surface-2 rounded p-2 font-mono text-[11px] border border-border space-y-1">
                  <div className="font-semibold">Fix:</div>
                  <div>1. Open the target page in Notion.</div>
                  <div>2. Click "•••" → "Add connections" → select your bot.</div>
                  <div>3. Try generating again.</div>
                </div>
              </div>
            )}

            <div className="flex flex-wrap items-center gap-1.5 rounded-lg bg-surface-2 px-3 py-2.5">
              <span className="text-[10px] font-bold uppercase tracking-wider text-ink-40 font-mono mr-1">Report:</span>
              {Object.entries(typeCounts).map(([type, count]) => (
                <span
                  key={type}
                  className={`rounded-full border border-border bg-surface-1 px-2 py-0.5 text-[10px] font-mono font-medium ${TYPE_ACCENT[type]?.split(" ")[1] ?? "text-ink-70"}`}
                >
                  {count} {TYPE_LABEL_PLURAL[type] ?? type}
                </span>
              ))}
              <span className="ml-auto text-[10px] font-mono text-ink-40">{new Date(batch.createdAt).toLocaleString()}</span>
            </div>

            <div className="space-y-2">
              {batch.items.map((item, idx) => {
                const expanded = expandedIdx === idx;
                const accent = TYPE_ACCENT[item.type] ?? "border-l-ink-40 text-ink-70";
                return (
                  <div
                    key={idx}
                    onClick={() => setExpandedIdx(expanded ? null : idx)}
                    className={`cursor-pointer rounded-md border border-border border-l-[3px] bg-surface-2 px-3 py-2.5 shadow-sm transition-all hover:border-border-strong hover:shadow-md ${accent}`}
                  >
                    <div className="flex items-center justify-between gap-2 text-[10px] font-bold uppercase tracking-wider font-mono mb-1">
                      <span className="truncate">{TYPE_LABEL[item.type] ?? item.type}</span>
                      <span className="shrink-0 text-ink-40">{expanded ? "▲" : "▼"}</span>
                    </div>
                    {item.title && <div className="text-sm font-semibold text-ink-100 mb-0.5">{item.title}</div>}
                    <div className="text-sm text-ink-100 leading-relaxed whitespace-pre-wrap">{item.text}</div>

                    {expanded && (
                      <div className="mt-2 grid grid-cols-2 gap-x-3 gap-y-1 border-t border-border pt-2 font-mono text-[10px] text-ink-40">
                        <span>Type: {item.type}</span>
                        <span>Characters: {item.text.length}</span>
                        <span>Date: {item.date ?? "—"}</span>
                        <span>Platform: {item.platform ?? "—"}</span>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {!batch && !error && !busy && (
          <div className="flex h-full flex-col items-center justify-center text-center text-xs text-ink-40 py-12 rounded-xl border border-dashed border-border">
            <div className="text-3xl mb-3 opacity-40">📡</div>
            <span className="font-semibold uppercase tracking-wider block mb-1 text-ink-70">Awaiting Instructions</span>
            Use a preset above, or tell the agent what you need.
          </div>
        )}
      </div>

      {/* Bottom dock: removed the redundant "Ready" / busy-label status
          span that used to sit here — the busy card above (in the scrollable
          view) already shows that, so repeating it here was redundant. */}
      <div className="shrink-0 border-t border-border bg-surface-1 p-3 space-y-2">
        <div className="flex flex-wrap items-center gap-x-2 gap-y-1.5">
          <PresetBar presets={presets} onRun={runPreset} busy={busy} />
          <BusinessBar selected={scopedBusiness} onSelect={setScopedBusiness} busy={busy} />
        </div>
        {scopedBusiness && (
          <div className="flex items-center gap-1.5 rounded-sm border border-accent/40 bg-accent-muted/30 px-2.5 py-1 text-[10px] font-mono uppercase tracking-wider text-ink-100">
            <span>Editing only: {scopedBusiness.name}</span>
            <button onClick={() => setScopedBusiness(null)} className="ml-auto text-ink-40 hover:text-ink-100">
              ✕
            </button>
          </div>
        )}
        <CommandBar
          onSubmit={runCommand}
          busy={busy}
          placeholder={
            scopedBusiness
              ? `Tell the agent what to change on "${scopedBusiness.name}" — e.g. "add 3 more quotes about customer loyalty"`
              : undefined
          }
        />
      </div>
    </div>
  );
}