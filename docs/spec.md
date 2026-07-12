# Notion Content Prompter — Architecture & Spec (v1)

**Type:** Cross-platform desktop utility (Tauri)
**Purpose:** A small, always-on-top "prompter" window that lets an agency generate AI content (headlines, sublines, quotes, tips, full monthly content calendars) directly into a dedicated Notion workspace for each client, with a pluggable AI backend.

---

## 1. Goals

- **Self-hosted, single-tenant per install.** You build and distribute the app (GitHub source + .dmg/.exe). Each client downloads and installs their own copy. There's no central "your app" managing many clients — each install is theirs, connected to their own Notion account, running independently.
- One click (or one prompt) generates a full month of content, structured into Notion pages/databases via a reusable **template** baked into the app.
- v1 ships with a single default AI model + a field for the client to paste in their own API key. Architecture leaves room for a v2 "agent marketplace" without a rewrite.
- Small footprint, fast-launching window — not a browser tab, not Electron bloat. Tauri fits this well (Rust core + native webview, installs are ~3-10MB vs Electron's 100MB+).
- Ships as signed installers for macOS (.dmg) and Windows (.exe/.msi), source on GitHub.

## 2. User Roles

There's really just **one role per install: the end user** (whoever installed the app — could be you testing it, or a client running their own copy). The app has no concept of "other clients" inside it — it only knows about *its own* connected Notion account and *its own* generated content. You (the agency) act as the software vendor, not an operator logged into a shared system.

If you personally want to use the app for several of your own accounts, you'd just run several installs (or, later, add a lightweight "switch workspace" toggle inside one install — but that's a nice-to-have, not required for v1).

## 3. High-Level Architecture

```
┌─────────────────────────────────────────────┐
│              Tauri Desktop Shell              │
│  ┌─────────────────────────────────────────┐ │
│  │   Frontend (React/Svelte + Tailwind)      │ │
│  │   - Prompter window (small, ~400x600)     │ │
│  │   - Tabs: Generate | Clients | AI Agent    │ │
│  │   - Calendar view                          │ │
│  └───────────────┬─────────────────────────┘ │
│                  │  Tauri IPC (invoke)         │
│  ┌───────────────▼─────────────────────────┐ │
│  │   Rust Core (Tauri backend)               │ │
│  │   - Secure credential store (OS keychain)  │ │
│  │   - Notion API client                      │ │
│  │   - AI provider adapter layer               │ │
│  │   - Local SQLite (client profiles, cache)   │ │
│  └───────────────┬─────────────┬────────────┘ │
└──────────────────┼─────────────┼──────────────┘
                   │             │
          ┌────────▼───┐   ┌────▼─────────────┐
          │ Notion API  │   │  AI Provider(s)   │
          │ (per client)│   │ (DeepSeek / BYO)  │
          └─────────────┘   └───────────────────┘
```

## 4. Tech Stack

| Layer | Choice | Why |
|---|---|---|
| Shell | **Tauri 2.x** | Native window, small binary, built-in secure storage, cross-platform signing |
| Frontend | React + TypeScript + Tailwind | Fast to build UI, huge ecosystem, easy for future contributors |
| Backend logic | Rust (Tauri commands) | Handles API calls, keychain access, file/DB I/O |
| Local storage | SQLite (via `rusqlite` or `sqlx`) | Stores client profiles, template configs, generation history — no external DB needed |
| Secrets | OS-native keychain (macOS Keychain / Windows Credential Manager) via `tauri-plugin-stronghold` or `keyring` crate | Notion tokens & AI API keys never touch plaintext disk |
| Notion integration | Official Notion REST API (OAuth2 for multi-client, or internal integration token per client as fallback) | |
| AI integration | Adapter pattern — start with DeepSeek API, generic OpenAI-compatible interface for BYO keys | Keeps door open for Claude, GPT, local models later |
| CI/CD | GitHub Actions + `tauri-action` | Builds signed .dmg + .exe/.msi on tag push |

## 5. Core Data Model

Since each install belongs to one user/workspace, there's no `Client` table — just local settings + generated content history.

```
Workspace (singleton — one per install)
 ├─ business_name, brand notes (used to personalize prompts)
 ├─ notion_workspace_id
 ├─ notion_access_token (encrypted, keychain-ref)
 └─ ai_config (see AgentConfig)

Template (ships built into the app; editable locally)
 ├─ id, name (e.g. "Standard Monthly Content Pack")
 ├─ notion_database_schema (headlines, sublines, quotes, tips, calendar)
 └─ prompt_blueprints (per content type, editable prompt templates)

ContentBatch
 ├─ id, month/year
 ├─ status (draft / pushed to Notion / archived)
 └─ items[] (headline/subline/quote/tip/calendar entries)

AgentConfig
 ├─ provider ("deepseek" | "byo")
 ├─ api_key (encrypted)
 ├─ base_url (for BYO OpenAI-compatible endpoints)
 └─ model_name

Preset
 ├─ id, label (e.g. "1 Week of Content", "1 Month of Content")
 ├─ prompt_template (string with {placeholders})
 └─ fields[] (e.g. "Business Name", "Tone", "Platform" — rendered as input boxes above the command bar)
```

## 6. Notion Integration

**Connecting Notion (one-time setup, done by whoever installed the app):**
1. On first launch, a **Settings** screen prompts: "Connect your Notion account."
2. **Confirmed approach for v1:** you create one Notion internal integration ("bot") under your own Notion developer account — something like *"[App Name] Content Bot."* The app ships with in-app instructions telling the user to: open the target Notion page → "Add connections" → select that bot → then paste the bot's integration token (or just confirm the connection, depending on Notion's flow) into the app's Settings screen.
3. This keeps the app fully self-hosted and flexible — it works against *any* Notion account, with zero backend or approval process on your end. (A full "Connect with Notion" OAuth button is nicer UX for non-technical users and can be added later, but requires registering and getting a public Notion integration reviewed — not needed to ship v1.)
4. Once connected, the app creates the **Template** structure directly inside that user's own workspace via the Notion API (`pages.create` for a parent page, `databases.create` for the four content databases + calendar database).

**Template structure created on first connect:**
- Parent page: `[Business Name] — Content Hub` (Business Name pulled from Workspace settings)
  - Database: **Headlines**
  - Database: **Sublines**
  - Database: **Quotes**
  - Database: **Useful Tips**
  - Database: **Content Calendar** (properties: Date, Content Type, Status, Linked Item, Platform)

**Pushing generated content:** After the AI generates a batch, the app writes rows into the corresponding Notion databases via `pages.create` calls, linking calendar entries to their source items via relation properties. Everything happens directly between the user's own installed app and their own Notion account — no data passes through anything you host.

## 7. AI Agent Layer

**v1 (as decided): one adapter, two presets of it**

Since your default is DeepSeek via **OpenRouter**, and OpenRouter exposes an OpenAI-compatible `/chat/completions` endpoint, there's no need for a separate "DeepSeek adapter" vs "BYO adapter" — it's **one generic OpenAI-compatible adapter**, just pointed at different credentials:

| | Base URL | API Key | Model |
|---|---|---|---|
| **Default (built-in)** | `https://openrouter.ai/api/v1` | your OpenRouter key, bundled/obfuscated in the app | `deepseek/deepseek-chat` (or whichever DeepSeek model you use) |
| **BYO** | user-entered (defaults to OpenRouter's URL as a convenience, editable) | user's own key | user-entered |

This means less code to maintain and a cleaner mental model in Settings: one "AI Provider" panel with a toggle — **Use built-in (DeepSeek)** vs **Use my own key** — rather than two separate integrations.

> Note: shipping your own OpenRouter key inside a distributed binary means a technically motivated user could extract it (it's not a server-side secret in this architecture). Reasonable mitigations: set a spend cap on that OpenRouter key, and/or route the default-model calls through a tiny proxy endpoint you control instead of embedding the raw key client-side (adds a small hosting requirement, but caps your exposure). Worth a quick decision before Phase 3 — flagged in §13.

**Adapter interface (so v2 doesn't require a rewrite):**
```rust
trait AiProvider {
    fn generate(&self, prompt: &str, params: GenParams) -> Result<String>;
}
```
Both the built-in and BYO configs implement this same trait via one `OpenAiCompatibleProvider` struct — swapping which credentials are active is a config change, not a code change.

**Deferred to v2 (not built now, but the trait above supports it later):**
- A "marketplace" tab listing vetted agents you approve.
- Local-model support with a hardware capability check (RAM/VRAM detection) before allowing install — this is the part conceptually similar to the "install into their system" idea you mentioned. Worth its own spec pass once v1 is validated, since it involves bundling or downloading model weights and is a meaningfully bigger scope (packaging, storage, GPU detection, license compliance).

## 8. Content Generation Pipeline

The core interaction is a **command bar**, not a single button — the agency (or client) can type anything ("write 10 quotes about morning routines and add them to the Quotes database," "restructure my calendar to skip weekends," "give me 3 headline options for Tuesday's post") and the AI agent interprets it and acts on the connected Notion workspace. Presets exist to remove the blank-page problem for people who don't want to write a prompt from scratch — they're shortcuts into the same command pipeline, not a separate mode.

1. **Command bar (primary):** Free-text input, always available. Whatever is typed goes to the AI agent along with context (client name, template schema, current calendar state) so it knows what it's allowed to touch in Notion.
2. **Presets (secondary, one click away):** A row/dropdown of built-in shortcuts:
   - "1 Week of Content"
   - "1 Month of Content"
   - "Headlines Only," "Quotes Only," "Tips Refresh," etc.
   Each preset renders a couple of quick fields above the command bar when selected — e.g. **Business Name**, **Tone**, **Platform(s)** — and fills those into a pre-written prompt template (`Preset.prompt_template`) behind the scenes. The user never has to see or write the actual prompt if they don't want to.
3. Whether typed freely or generated from a preset, the final prompt goes to the active `AiProvider`, which returns structured content (parsed into headline/subline/quote/tip/calendar-entry sections).
4. Results shown in-app for quick edit/approve before anything is pushed — this stays true whether the request came from a preset or a free command, so nothing lands in the client's live Notion workspace unreviewed.
5. On approve, batch is written to Notion (§6) and marked `pushed`.
6. Agency admins can save any successful free-form command as a **new custom preset** for that client (or globally), so one-off requests turn into reusable shortcuts over time.

## 9. UI/UX — The "Small Window"

- Default size: compact (~420×640), resizable, remembers last position (OS-native via Tauri window state plugin).
- Tabs (per your ask):
  1. **Generate** — a command bar for free-form requests to the AI agent, plus a row of one-click presets ("1 Week of Content," "1 Month of Content," etc.) that pop a couple of quick fields (Business Name, Tone, Platform) instead of making the user write a prompt
  2. **Calendar** — visual month view of what's been generated/pushed
  3. **Settings** — connect/reconnect Notion, set Business Name & brand notes, manage the AI agent (default vs BYO key)
- No client list, no switching profiles — one install, one Notion workspace, one set of settings. If you (the agency) want to work across several of your own accounts, you'd run separate installs or, later, add an optional "switch workspace" toggle — not required for v1.
- **Visual style (confirmed):** clean, dark, neutral. Near-black background (`#111214`-ish), dark gray surfaces/cards (`#1c1d20`, `#2a2b2f`), light gray/off-white text, one restrained accent color for buttons/active states (a muted blue or green reads "professional tool" rather than "flashy app" — happy to lock in an exact accent once you see it). No client-specific branding baked into the shared UI chrome; the only per-workspace personalization is the Business Name shown in headers and used in prompts.

## 10. Distribution

- **Repo:** public or private GitHub repo, `main` branch protected, releases tagged `vX.Y.Z`.
- **CI:** GitHub Actions workflow using `tauri-apps/tauri-action` — on tag push, builds:
  - macOS: `.dmg` (needs Apple Developer ID for notarization — without it, users get a Gatekeeper warning they have to click through)
  - Windows: `.msi`/`.exe` (code-signing cert recommended but not required to run; unsigned just triggers a SmartScreen warning)
- Auto-update: Tauri's built-in updater plugin can point at your GitHub Releases feed so clients get pushed updates without manual reinstalls.

## 11. Licensing (Simple License Key)

Since there's no phone-home for user data, license enforcement needs its own lightweight mechanism, decoupled from the app's actual functionality:

- **Approach:** signed license keys. You generate a keypair once; a small script (yours, run locally whenever you sell a copy) signs a payload like `{ "issued_to": "Client Name", "issued_at": "..." }` and outputs a license key string.
- **On app launch:** the app checks the license key's signature **entirely offline**, using the public key baked into the binary. No network call required, no server to maintain — this fits the "fully self-hosted" philosophy while still gating usage.
- **What this does and doesn't protect against:** a valid key can technically be copy-pasted between machines (there's no hardware lock unless you add one). For a simple deterrent against casual redistribution, plain signature-checking is usually enough; if you later want to *revoke* a specific client's access remotely, that requires a small hosted check-in endpoint instead of pure offline validation — a bigger step, only worth it if piracy becomes an actual problem.
- **Recommended v1 scope:** offline signature check only, no revocation, no expiry (or optional expiry date baked into the signed payload if you want to sell subscriptions rather than one-time licenses).

## 12. Security Notes

- The Notion token and AI API key for an install are **never stored in plaintext** — use OS keychain via Tauri's secure storage, referenced by ID in the local SQLite DB.
- Because each install is single-tenant and self-hosted, there's no cross-client data exposure risk by design — everything lives on that one machine, talking directly to that one Notion account. Nothing routes through infrastructure you host, which also means you have no ongoing hosting cost or liability for client data.
- BYO API keys are the client's own responsibility/cost — make this clear in-app so there's no billing confusion between "your DeepSeek default" and "their own key."
- The bundled OpenRouter key (§7) is the one meaningful secret shipped inside the binary — treat it as semi-public (spend-capped) rather than as a fully protected server-side secret.

## 13. Suggested Build Order (Phases)

1. **Phase 1 — Skeleton:** Tauri + React shell, tabbed window (Generate / Calendar / Settings), dark neutral theme, local SQLite for settings + content history, no AI/Notion yet.
2. **Phase 2 — Notion:** Settings flow for connecting the Notion bot, template creation in the user's own workspace on first connect.
3. **Phase 3 — AI:** `OpenAiCompatibleProvider` wired up with built-in OpenRouter/DeepSeek credentials + BYO override, command bar wired to it.
4. **Phase 4 — Presets + calendar + push:** Built-in presets (1 Week / 1 Month), generate → review → push-to-Notion flow, calendar view.
5. **Phase 5 — Licensing:** Offline signature-check license gate, key-generation script for your own use when selling a copy.
6. **Phase 6 — Packaging:** GitHub Actions build pipeline, first releases for Mac/Windows — this is the artifact you'll actually hand to clients.
7. **Phase 7 (later):** Optional OAuth-based Notion connect for a smoother non-technical setup, agent marketplace, local-model support.

## 14. Remaining Open Questions

- Exact **accent color** for the dark theme (a muted blue, green, or something else on-brand)?
- Rough **spend cap** you're comfortable putting on the bundled OpenRouter key, since it ships inside the binary?
- **License model:** one-time key per client, or would you rather sell it as a recurring thing (which would mean baking an expiry date into the signed license)?
- Do you want the app to show your own name/logo anywhere ("Powered by [Your Agency]"), or should it be fully white-label with no trace of who built it?
