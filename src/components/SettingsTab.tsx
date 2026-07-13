import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { ModelInfo, OllamaStatus, WorkspaceSettings } from "../lib/types";

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="flex flex-col gap-3 rounded-sm border border-border bg-surface-1 p-3">
      <h2 className="font-mono text-xs uppercase tracking-widest text-ink-40">{title}</h2>
      {children}
    </section>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-xs text-ink-70">{label}</span>
      {children}
    </label>
  );
}

const inputClass =
  "rounded-sm border border-border bg-surface-2 px-2.5 py-1.5 text-sm text-ink-100 placeholder:text-ink-40 outline-none focus:border-accent";

const secondaryButtonClass =
  "rounded-sm border border-border bg-surface-2 px-2.5 py-1 text-xs text-ink-100 hover:border-border-strong disabled:opacity-40";

interface SettingsTabProps {
  settings: WorkspaceSettings | null;
  onSettingsChange: (settings: WorkspaceSettings) => void;
  refreshSettings: () => Promise<void>;
}

export default function SettingsTab({ settings, onSettingsChange, refreshSettings }: SettingsTabProps) {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [ollama, setOllama] = useState<OllamaStatus | null>(null);
  const [notionToken, setNotionToken] = useState("");
  const [byoKey, setByoKey] = useState("");
  const [unsplashKey, setUnsplashKey] = useState("");
  const [byoModelDraft, setByoModelDraft] = useState(settings?.byoModel ?? "");
  const [status, setStatus] = useState<string | null>(null);
  const [notionTestStatus, setNotionTestStatus] = useState<string | null>(null);
  const [aiTestStatus, setAiTestStatus] = useState<string | null>(null);
  const [testingNotion, setTestingNotion] = useState(false);
  const [testingAi, setTestingAi] = useState(false);
  const [pullingId, setPullingId] = useState<string | null>(null);
  const [manualAgentEditOpen, setManualAgentEditOpen] = useState<boolean | null>(null);
  // Same collapse/edit pattern as the AI Agent section: null = "auto"
  // (open the key field until one's saved, then collapse to a summary).
  const [manualUnsplashEditOpen, setManualUnsplashEditOpen] = useState<boolean | null>(null);
  const [manualNotionEditOpen, setManualNotionEditOpen] = useState(false);

  const refreshModels = () => {
    api.getAvailableModels().then(setModels).catch(() => setModels([]));
    api.getOllamaStatus().then(setOllama).catch(() => setOllama(null));
  };

  const localModels = models.filter((m) => m.source === "local");

  useEffect(() => {
    refreshModels();
  }, []);

  useEffect(() => {
    setByoModelDraft(settings?.byoModel ?? "");
  }, [settings?.byoModel]);

  useEffect(() => {
    if (!pullingId) return;
    const interval = setInterval(async () => {
      const s = await api.getOllamaStatus();
      setOllama(s);
      if (s.pulledModels.includes(pullingId)) {
        setPullingId(null);
        refreshModels();
      }
    }, 4000);
    return () => clearInterval(interval);
  }, [pullingId]);

  if (!settings) {
    return <div className="flex h-full items-center justify-center text-sm text-ink-40">Loading settings…</div>;
  }

  // Fixed: this type now matches api.ts's saveSettings signature exactly
  // (byoApiKey AND unsplashApiKey) — the mismatch between this local type
  // and api.ts's is exactly what caused the TS2353 build error.
  const patch = async (p: Partial<WorkspaceSettings> & { byoApiKey?: string; unsplashApiKey?: string }) => {
    const updated = await api.saveSettings(p);
    onSettingsChange(updated);
    return updated;
  };

  const connectNotion = async () => {
    setStatus("Connecting…");
    try {
      const res = await api.connectNotion(notionToken);
      setStatus(res.message);
      if (res.ok) {
        await refreshSettings();
        setNotionToken("");
        setManualNotionEditOpen(false);
      }
    } catch (err) {
      setStatus(String(err));
    }
  };

  const testNotion = async () => {
    setTestingNotion(true);
    setNotionTestStatus(null);
    try {
      const res = await api.testNotionConnection();
      setNotionTestStatus(res.ok ? `✓ ${res.message}` : `✗ ${res.message}`);
    } catch (err) {
      setNotionTestStatus(`✗ ${String(err)}`);
    } finally {
      setTestingNotion(false);
    }
  };

  const testAi = async () => {
    setTestingAi(true);
    setAiTestStatus(null);
    try {
      const res = await api.testAiConnection();
      setAiTestStatus(res.ok ? `✓ ${res.message}` : `✗ ${res.message}`);
    } catch (err) {
      setAiTestStatus(`✗ ${String(err)}`);
    } finally {
      setTestingAi(false);
    }
  };

  const downloadModel = async (id: string) => {
    setPullingId(id);
    try {
      const res = await api.pullOllamaModel(id);
      setStatus(res.message);
    } catch (err) {
      setStatus(String(err));
      setPullingId(null);
    }
  };

  const saveByoKey = async () => {
    await patch({ byoApiKey: byoKey });
    setByoKey("");
  };

  const saveUnsplashKey = async () => {
    await patch({ unsplashApiKey: unsplashKey });
    setUnsplashKey("");
    setManualUnsplashEditOpen(false);
  };

  const selectedLocal = localModels.find((m) => m.id === settings.selectedModelId);
  const needsDownload = selectedLocal?.status === "pull_required";

  const agentConnected =
    (settings.aiProvider === "local" && selectedLocal?.status === "ready") ||
    (settings.aiProvider === "byo" && settings.byoKeySet && !!settings.byoModel);

  const agentSummaryLabel =
    settings.aiProvider === "local"
      ? selectedLocal?.label ?? "No local agent selected"
      : settings.byoModel
      ? `${settings.byoModel} · custom API key`
      : "No API key set";

  const agentEditOpen = manualAgentEditOpen ?? !agentConnected;
  const unsplashEditOpen = manualUnsplashEditOpen ?? !settings.unsplashKeySet;
  const testButtonLabel = settings.aiProvider === "byo" ? "Test API key" : "Test connection";

  const providerButtonClass = (id: WorkspaceSettings["aiProvider"]) =>
    `flex-1 rounded-sm border px-3 py-2 text-xs font-medium ${
      settings.aiProvider === id ? "border-accent bg-accent-muted text-ink-100" : "border-border bg-surface-2 text-ink-70"
    }`;

  return (
    <div className="flex h-full flex-col gap-4 overflow-y-auto p-4">
      <Section title="Notion">
        {settings.notionConnected && !manualNotionEditOpen ? (
          <div className="flex items-center justify-between">
            <span className="text-sm text-emerald-400">Connected ✓</span>
            <div className="flex gap-2">
              <button onClick={testNotion} disabled={testingNotion} className={secondaryButtonClass}>
                {testingNotion ? "Testing…" : "Test connection"}
              </button>
              <button onClick={() => setManualNotionEditOpen(true)} className={secondaryButtonClass}>
                Edit
              </button>
            </div>
          </div>
        ) : (
          <>
            <p className="text-xs text-ink-70">
              1. Open the target page in Notion → <strong>Add connections</strong> → select your
              Content Bot integration.
              <br />
              2. Paste its integration token below.
            </p>
            <Field label="Notion integration token">
              <input
                className={inputClass}
                type="password"
                value={notionToken}
                onChange={(e) => setNotionToken(e.target.value)}
                placeholder="secret_..."
              />
            </Field>
            <div className="flex gap-2">
              <button
                onClick={connectNotion}
                disabled={!notionToken.trim()}
                className="self-start rounded-sm bg-accent px-3 py-1.5 text-xs font-medium text-white hover:bg-accent-hover disabled:opacity-40"
              >
                {settings.notionConnected ? "Reconnect" : "Connect"}
              </button>
              {settings.notionConnected && (
                <button
                  onClick={() => {
                    setManualNotionEditOpen(false);
                    setNotionToken("");
                  }}
                  className="text-xs text-ink-40 underline hover:text-ink-100"
                >
                  Cancel
                </button>
              )}
            </div>
          </>
        )}
        {notionTestStatus && <p className="text-xs text-ink-70">{notionTestStatus}</p>}
      </Section>

      <Section title="AI Agent">
        {!agentEditOpen && (
          <div className="flex items-center justify-between gap-2">
            <div className="flex flex-col">
              <span className="text-sm text-emerald-400">Connected ✓</span>
              <span className="text-xs text-ink-40">{agentSummaryLabel}</span>
            </div>
            <div className="flex shrink-0 gap-2">
              <button onClick={testAi} disabled={testingAi} className={secondaryButtonClass}>
                {testingAi ? "Testing…" : testButtonLabel}
              </button>
              <button onClick={() => setManualAgentEditOpen(true)} className={secondaryButtonClass}>
                Edit
              </button>
            </div>
          </div>
        )}

        {agentEditOpen && (
          <>
            <div className="flex gap-2">
              <button onClick={() => patch({ aiProvider: "local" })} className={providerButtonClass("local")}>
                Local (Ollama)
              </button>
              <button onClick={() => patch({ aiProvider: "byo" })} className={providerButtonClass("byo")}>
                My own API key
              </button>
            </div>

            {settings.aiProvider === "local" && (
              <>
                <Field label="Local agent">
                  <select
                    className={inputClass}
                    value={settings.selectedModelId}
                    onChange={(e) => patch({ aiProvider: "local", selectedModelId: e.target.value })}
                  >
                    <option value="" disabled>
                      Choose an agent…
                    </option>
                    {localModels.some((m) => m.status === "ready") && (
                      <optgroup label="Installed on this machine">
                        {localModels
                          .filter((m) => m.status === "ready")
                          .map((m) => (
                            <option key={m.id} value={m.id}>
                              {m.label} {m.recommended ? "★" : ""}
                            </option>
                          ))}
                      </optgroup>
                    )}
                    {localModels.some((m) => m.status !== "ready") && (
                      <optgroup label="Available to download">
                        {localModels
                          .filter((m) => m.status !== "ready")
                          .map((m) => (
                            <option key={m.id} value={m.id}>
                              {m.label} {m.recommended ? "★" : ""} —{" "}
                              {m.status === "pull_required"
                                ? "needs download"
                                : m.status === "insufficient_ram"
                                ? "not enough RAM"
                                : "install Ollama"}
                            </option>
                          ))}
                      </optgroup>
                    )}
                  </select>
                </Field>

                {ollama !== null && !ollama.installed && (
                  <div className="flex flex-col gap-2 rounded-sm border border-red-900 bg-red-950/30 p-2.5">
                    <p className="text-xs text-red-300">Ollama isn't installed yet — required to run agents locally.</p>
                    <a href="https://ollama.com/download" target="_blank" rel="noreferrer" className="text-xs text-red-300 underline">
                      Download Ollama →
                    </a>
                    <button onClick={refreshModels} className="self-start text-xs text-ink-40 underline hover:text-ink-100">
                      I installed it — recheck
                    </button>
                  </div>
                )}

                {ollama?.installed && needsDownload && (
                  <button
                    onClick={() => downloadModel(settings.selectedModelId)}
                    disabled={pullingId === settings.selectedModelId}
                    className="self-start rounded-sm border border-accent bg-accent-muted px-3 py-1.5 text-xs font-medium text-ink-100 disabled:opacity-50"
                  >
                    {pullingId === settings.selectedModelId ? "Downloading…" : `Download "${settings.selectedModelId}"`}
                  </button>
                )}

                <a href="https://ollama.com/library" target="_blank" rel="noreferrer" className="text-xs text-accent underline">
                  Browse more agents / models (Ollama library) →
                </a>
              </>
            )}

            {settings.aiProvider === "byo" && (
              <div className="flex flex-col gap-2">
                <Field label="Model ID">
                  <input
                    className={inputClass}
                    value={byoModelDraft}
                    onChange={(e) => setByoModelDraft(e.target.value)}
                    onBlur={() => patch({ byoModel: byoModelDraft })}
                    placeholder="deepseek/deepseek-v4-flash"
                  />
                </Field>
                <Field label={`API key ${settings.byoKeySet ? "(set — leave blank to keep)" : ""}`}>
                  <input
                    className={inputClass}
                    type="password"
                    value={byoKey}
                    onChange={(e) => setByoKey(e.target.value)}
                    placeholder="sk-..."
                  />
                </Field>
                <div className="flex flex-col gap-1 text-xs text-ink-40">
                  <a href="https://openrouter.ai/models" target="_blank" rel="noreferrer" className="text-accent underline">
                    Find a Model ID →
                  </a>
                  <a href="https://openrouter.ai/keys" target="_blank" rel="noreferrer" className="text-accent underline">
                    Get an API key →
                  </a>
                </div>
                <div className="flex gap-2">
                  <button
                    onClick={saveByoKey}
                    disabled={!byoKey.trim()}
                    className="rounded-sm border border-border bg-surface-2 px-3 py-1.5 text-xs font-medium text-ink-100 hover:border-border-strong disabled:opacity-40"
                  >
                    Save key
                  </button>
                  <button
                    onClick={testAi}
                    disabled={testingAi || !settings.byoKeySet}
                    className="rounded-sm border border-border bg-surface-2 px-3 py-1.5 text-xs font-medium text-ink-100 hover:border-border-strong disabled:opacity-40"
                  >
                    {testingAi ? "Testing…" : "Test API key"}
                  </button>
                </div>
              </div>
            )}

            <div className="flex items-center justify-between border-t border-border pt-2">
              <button onClick={testAi} disabled={testingAi} className={secondaryButtonClass}>
                {testingAi ? "Testing…" : testButtonLabel}
              </button>
              {agentConnected && (
                <button onClick={() => setManualAgentEditOpen(false)} className="text-xs text-ink-40 underline hover:text-ink-100">
                  Done
                </button>
              )}
            </div>
          </>
        )}

        {aiTestStatus && <p className="text-xs text-ink-70">{aiTestStatus}</p>}
      </Section>

      {/* Now matches the Notion/AI Agent pattern: collapses to a
          "Connected ✓" summary with an Edit button once a key is saved,
          instead of always showing the input field. */}
      <Section title="Cover Images">
        {!unsplashEditOpen ? (
          <div className="flex items-center justify-between gap-2">
            <span className="text-sm text-emerald-400">Connected ✓</span>
            <button onClick={() => setManualUnsplashEditOpen(true)} className={secondaryButtonClass}>
              Edit
            </button>
          </div>
        ) : (
          <>
            <p className="text-xs text-ink-70">
              When set, business pages get an AI-picked, context-relevant cover photo from Unsplash. Without a
              key, pages are created with no cover — a plain page instead of a random stock photo.
            </p>
            <Field label={`Unsplash API key ${settings.unsplashKeySet ? "(set — leave blank to keep)" : ""}`}>
              <input
                className={inputClass}
                type="password"
                value={unsplashKey}
                onChange={(e) => setUnsplashKey(e.target.value)}
                placeholder="your Unsplash access key"
              />
            </Field>
            <div className="flex items-center justify-between">
              <a href="https://unsplash.com/developers" target="_blank" rel="noreferrer" className="text-xs text-accent underline">
                Get a free key →
              </a>
              <div className="flex gap-2">
                {settings.unsplashKeySet && (
                  <button onClick={() => setManualUnsplashEditOpen(false)} className="text-xs text-ink-40 underline hover:text-ink-100">
                    Cancel
                  </button>
                )}
                <button onClick={saveUnsplashKey} disabled={!unsplashKey.trim()} className={secondaryButtonClass}>
                  Save key
                </button>
              </div>
            </div>
          </>
        )}
      </Section>

      {status && <p className="font-mono text-xs text-ink-40">{status}</p>}
    </div>
  );
}