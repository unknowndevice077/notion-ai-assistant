import { useState, KeyboardEvent } from "react";

interface CommandBarProps {
  onSubmit: (command: string) => void;
  busy: boolean;
  placeholder?: string;
}

/**
 * The primary interaction surface: type anything, the AI agent acts on it.
 * Presets (see PresetBar) are shortcuts into this same pipeline, not a
 * separate mode — so this bar stays the one source of truth for "what
 * are we asking the agent to do." When a business is selected in
 * BusinessBar, this same box is what you type the edit instruction into.
 */
export default function CommandBar({ onSubmit, busy, placeholder }: CommandBarProps) {
  const [value, setValue] = useState("");

  const submit = () => {
    const trimmed = value.trim();
    if (!trimmed || busy) return;
    onSubmit(trimmed);
    setValue("");
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  };

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-end gap-2 rounded-lg border border-border bg-surface-2 p-2 focus-within:border-accent transition-colors">
        <textarea
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={handleKeyDown}
          disabled={busy}
          rows={2}
          placeholder={placeholder ?? "Tell the agent what to do — e.g. “write 10 quotes about morning routines and add them to Quotes”"}
          className="flex-1 resize-none bg-transparent text-sm text-ink-100 placeholder:text-ink-40 outline-none disabled:opacity-50"
        />
        <button
          onClick={submit}
          disabled={busy || !value.trim()}
          className="shrink-0 rounded-md bg-accent px-3 py-2 text-sm font-medium text-white transition-colors hover:bg-accent-hover disabled:cursor-not-allowed disabled:opacity-40"
        >
          {busy ? "Working…" : "Send"}
        </button>
      </div>
      <p className="text-xs text-ink-40">
        Enter to send · Shift+Enter for a new line
      </p>
    </div>
  );
}