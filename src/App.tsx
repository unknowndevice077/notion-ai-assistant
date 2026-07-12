import { useCallback, useEffect, useState } from "react";
import { api } from "./lib/api";
import type { WorkspaceSettings } from "./lib/types";
import GenerateTab from "./components/GenerateTab";
import SettingsTab from "./components/SettingsTab";

type Tab = "generate" | "settings";

const TABS: { id: Tab; label: string }[] = [
  { id: "generate", label: "GENERATE" },
  { id: "settings", label: "SETTINGS" },
];

export default function App() {
  const [tab, setTab] = useState<Tab>("generate");
  const [settings, setSettings] = useState<WorkspaceSettings | null>(null);

  const refreshSettings = useCallback(() => {
    return api.getSettings().then(setSettings);
  }, []);

  useEffect(() => {
    refreshSettings();
  }, [refreshSettings]);

  useEffect(() => {
    document.title = "Notion AI Assistant";
  }, []);

  return (
    <div className="flex h-screen w-screen flex-col bg-surface-0 bg-grid font-sans">
      <nav className="flex shrink-0 gap-1 border-b border-border px-3 py-2">
        {TABS.map((t) => (
          <button
            key={t.id}
            onClick={() => setTab(t.id)}
            className={`rounded-sm px-3 py-1.5 font-mono text-xs tracking-wider transition-colors disabled:cursor-not-allowed disabled:opacity-30 ${
              tab === t.id ? "bg-surface-2 text-accent" : "text-ink-70 hover:bg-surface-2 hover:text-ink-100"
            }`}
          >
            {t.label}
          </button>
        ))}
      </nav>

      {/* Both tabs stay mounted at all times and are just shown/hidden —
          this is what keeps in-progress typing, generated batches, and
          Settings' already-fetched Ollama/model state intact when you
          switch tabs, instead of losing it to a full unmount/remount. */}
      <main className="min-h-0 flex-1">
        <div className={tab === "generate" ? "h-full" : "hidden"}>
          <GenerateTab />
        </div>
        <div className={tab === "settings" ? "h-full" : "hidden"}>
          <SettingsTab settings={settings} onSettingsChange={setSettings} refreshSettings={refreshSettings} />
        </div>
      </main>
    </div>
  );
}