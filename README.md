# Content Prompter

A small, self-hosted desktop window that lets an AI agent generate content
(headlines, sublines, quotes, tips, full content calendars) directly into a
Notion workspace — command-bar driven, with one-click presets for people who
don't want to write a prompt.

Each install is single-tenant: whoever installs it connects **their own**
Notion account and (optionally) their own AI key. Nothing routes through
infrastructure you host.

See [`docs/spec.md`](./docs/spec.md) if you want the full architecture
rationale this was built from — this README is just "how to run it."

## Stack

- **Shell:** Tauri 2 (Rust core + native webview)
- **Frontend:** React + TypeScript + Tailwind (dark neutral theme)
- **Local storage:** SQLite (settings, presets, content history) + OS keychain
  (Notion token, AI API key — never stored in plaintext)
- **AI:** one OpenAI-compatible adapter, used both for the built-in default
  (DeepSeek via OpenRouter) and for a client's own key
- **Notion:** internal integration ("bot") token, pasted in by the user —
  no OAuth app review needed for v1

## Getting set up locally

```bash
npm install
npm run tauri dev
```

Requirements: Node 20+, Rust (stable, via rustup), and the platform's Tauri
prerequisites (Xcode command line tools on macOS; WebView2 + MSVC build
tools on Windows — see Tauri's own "Prerequisites" docs for your OS, since
these change independently of this app).

## One-time setup before you can actually generate/push content

1. **Create your Notion integration ("bot")** at
   https://www.notion.so/my-integrations — name it something like
   "Content Prompter Bot," internal integration, no OAuth needed. Copy its
   **Internal Integration Token**.
2. In the app's **Settings** tab, paste that token to connect, then use
   **"Test connection"** to confirm it's actually working.
3. In Notion itself, share the page you want the content hub created under
   with that same bot (••• menu → Add connections → your bot).
4. Back in **Settings**, type a hub name (e.g. "Acme Studio — Content Hub")
   and click **"(Re)create content template"** — creates the hub page with
   the five databases (Headlines, Sublines, Quotes, Useful Tips, Content
   Calendar). Note there's no persistent "business name" setting — the hub
   name is just entered when you (re)create the template, and any other
   business context goes into a preset or the command bar at generation
   time, which is more flexible per-request than a fixed global field.
5. In **Settings → AI agent**, pick a model from the dropdown (defaults to
   a flexible cloud model — currently DeepSeek, swappable to Qwen/GPT-4o
   mini/etc. with one click) or switch to "Use my own key." Use
   **"Test connection"** there too before relying on it.

Every client who gets a copy of this app repeats steps 1–5 with their own
Notion account (or you build them their own bot integration if you'd rather
control that centrally — either works, since the token is only used by
their own local install).

## Generating content

The **Generate** tab has two ways in, both hitting the same pipeline:

- **Command bar:** type anything — "write 10 quotes about morning routines
  and add them to Quotes," "give me 3 headline options for Tuesday."
- **Presets:** click the "Presets ▲" button above the command bar — it pops
  open a menu (built-ins: 1 Week of Content, 1 Month of Content, Headlines
  Only, Quotes Only, Tips Refresh). Pick one, fill in the couple of fields
  it asks for (Business Name, Tone, sometimes Platform), and run — no need
  to write a prompt from scratch.

There's no calendar view inside the app on purpose — calendar entries still
generate and push like everything else, they just live in the Content
Calendar database in Notion, not as a separate in-app screen.

## AI model catalog

`src-tauri/src/model_registry.rs` lists the models offered in Settings:

- **Cloud models** (DeepSeek, Qwen, GPT-4o mini) — all run through the
  built-in OpenRouter connection or a client's own key, and are fully
  functional today.
- **Local models** (Qwen/DeepSeek via Ollama) — shown with a compatibility
  badge based on the machine's RAM (no GPU/VRAM check yet), but are
  informational only for now — actual local execution is a v2 feature, not
  wired up yet. See "Known follow-ups" below.

Swapping the *default* cloud model, or adding another one to the list, is a
one-line change to the `CATALOG` array — no other code needs to change,
since every cloud entry speaks the same OpenAI-compatible API.

## Licensing & the built-in AI key

License keys are **signed offline** — no server, no phone-home:

```bash
node scripts/generate-keypair.mjs        # once, ever — keep the output private
node scripts/generate-license.mjs "Client Name" 2027-01-01   # per client, expiry optional
```

Paste the printed **public key** into
`src-tauri/src/license.rs` → `LICENSE_PUBLIC_KEY_B64` before building the
version you distribute. Give each client the **license key** string printed
by `generate-license.mjs` to paste into Settings → License.

The built-in AI default (DeepSeek via OpenRouter) needs its API key baked in
at build time so clients don't need their own key just to try the app:

```bash
BUILT_IN_OPENROUTER_KEY=sk-or-... npm run tauri build
```

Since this key ships inside a binary you're handing out, **put a spend cap
on it** in OpenRouter's dashboard — treat it as semi-public, not a protected
server-side secret.

## Building installers

```bash
npm run tauri build
```

Or push a tag (`git tag v0.1.0 && git push --tags`) to let
`.github/workflows/release.yml` build both macOS (.dmg) and Windows
(.msi/.exe) automatically. Set the `BUILT_IN_OPENROUTER_KEY` repo secret so
the CI build has it available.

You'll also need real app icons before a release build — Tauri won't bundle
without them:

```bash
npm run tauri icon path/to/your-1024x1024-icon.png
```

## License

This project is released under the MIT License. See [`LICENSE`](./LICENSE)
for the full terms.

## Known follow-ups (called out honestly, not hidden)

- **Notion database ID caching:** `push_batch_to_notion` currently re-lists
  the hub page's child databases on every push to find their IDs. Fine at
  this scale; if pushes start feeling slow, cache the five IDs in SQLite
  after template creation instead.
- **OAuth-based Notion connect** (nicer for non-technical clients) isn't
  built — v1 intentionally uses the simpler paste-a-token flow. See the
  spec doc, §6.
- **Local model execution:** the model dropdown shows local models with a
  RAM-based compatibility badge, but selecting one doesn't actually download
  or run anything yet — only cloud models (via the built-in key or BYO) are
  functional today. Turning this on means packaging/downloading model
  weights, real hardware detection (GPU/VRAM, not just RAM), and picking a
  local runtime (e.g. Ollama) to shell out to — a meaningfully bigger build,
  deliberately deferred.
- **License revocation:** current design is offline verify-only (no way to
  remotely kill a key once issued). Adding that requires a small hosted
  check-in endpoint — a deliberate tradeoff for staying phone-home-free in
  v1.
