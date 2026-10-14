# Notion AI Assistant

A small desktop app, built for a content agency client, that turns one prompt into a dated content calendar inside Notion: headlines, quotes and tips for each client business, with a matching cover photo, ready to post. No Notion setup by hand.

Built with **Rust + Tauri 2** and a **React + TypeScript** UI. It runs with a **local Ollama model** (no API key, nothing leaves the machine except the Notion push) or with **your own API key** for a cloud model.

## What it does

- **One prompt → organized Notion pages.** Describe what you want, or fill in a preset (business name, context, how many days, how many headlines/quotes/tips per day, start date). The app generates the content, builds the pages and databases in your Notion workspace, and dates each item on the calendar.
- **One workspace per client business.** Switch between businesses from the top bar; each keeps its own context, so the content stays on-brand. Businesses deleted in Notion are dropped from the list on the next load, instead of lingering as stale entries.
- **Cover photos.** The model suggests a short photo search for each business and the app pulls a matching image from Unsplash (optional, needs a free Unsplash key).
- **Review before it's final.** Every run is saved as a batch you can list, and a push can be discarded.
- **Local or cloud AI.** Pick an installed Ollama model, or pull one from inside the app. Or choose "My own API key" and paste a key for an OpenAI-compatible provider. Both connections can be tested from Settings.

## Security

- The Notion token, AI key and Unsplash key are stored in the **OS keyring** (Credential Manager on Windows, Keychain on macOS, Secret Service on Linux), never in a file. There is no `.env`.
- The local SQLite database holds settings, presets and batch history, not secrets.
- The webview runs under a restrictive Content Security Policy (`default-src 'self'`).
- With Ollama, prompts and generated content never leave the machine except when pushed to your own Notion workspace.

## Install

Download the latest Windows or macOS installer from [Releases](https://github.com/unknowndevice077/notion-ai-assistant/releases). Builds are produced by GitHub Actions whenever a `v*.*.*` tag is pushed.

### First run

1. Create a Notion integration at https://www.notion.com/my-integrations, copy its token, and share the pages you want it to write to with the integration.
2. Open the app → **Settings** → paste the token and press **Test**.
3. Under **AI Agent**, pick a local Ollama model (install [Ollama](https://ollama.com) first) or choose "My own API key" and paste your key.
4. Optional: paste an Unsplash access key for cover photos.

## Build from source

Requires Rust (stable), Node.js 18+ and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.

```bash
git clone https://github.com/unknowndevice077/notion-ai-assistant.git
cd notion-ai-assistant
npm install
npm run tauri dev      # run in development
npm run tauri build    # build an installer for this OS
```

## Project structure

```
├── src/                     React + TypeScript UI
│   ├── App.tsx
│   └── components/          GenerateTab, PresetBar, BusinessBar, CommandBar, SettingsTab
├── src-tauri/
│   ├── src/
│   │   ├── main.rs          Tauri setup, command registration
│   │   ├── commands.rs      Commands the UI calls (generate, presets, businesses, settings)
│   │   ├── ai.rs            Prompt building and parsing the model's JSON output
│   │   ├── ollama.rs        Local model status, listing and pulling
│   │   ├── model_registry.rs  Available models per provider
│   │   ├── notion.rs        Notion API client: pages, databases, calendar entries
│   │   ├── unsplash.rs      Cover photo search
│   │   ├── db.rs            SQLite: settings, presets, batches
│   │   └── models.rs        Shared types
│   └── tauri.conf.json
├── docs/spec.md             Architecture and design decisions
└── .github/workflows/release.yml   Windows + macOS installers on tag push
```

## License

Apache License 2.0. See [LICENSE](LICENSE).
