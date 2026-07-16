# Notion AI Assistant

A simple, self-hosted Notion AI Assistant built with Rust and Tauri. This desktop application provides a small-window content prompter that integrates AI capabilities with Notion, running as a self-contained desktop application.

## Features

- **AI-Powered Content Prompting** - Get intelligent suggestions while working
- **Notion Integration** - Seamless integration with Notion workspace
- **Small Window Interface** - Compact, always-available assistant window
- **Self-Hosted** - Run locally on your machine, no cloud dependencies
- **Fast & Lightweight** - Built with Rust for performance
- **Cross-Platform** - Works on Windows, macOS, and Linux
- **Secure** - Credentials stored securely with keyring integration
- **Local Storage** - SQLite database for persistent storage
- **Real-time Updates** - Async request handling with Tokio
- **Desktop App** - Native Tauri application

## Tech Stack

- **Frontend**: Tauri 2, Web Technologies (HTML/CSS/JS)
- **Backend**: Rust 2021 Edition
- **Desktop Framework**: Tauri 2
- **Async Runtime**: Tokio with full features
- **Database**: SQLite with rusqlite
- **HTTP Client**: Reqwest with JSON support
- **Serialization**: Serde with JSON support
- **Security**: Keyring for credential storage
- **UUID**: uuid v4 generation
- **System Info**: sysinfo for hardware monitoring
- **Error Handling**: thiserror
- **Date/Time**: Chrono with serde support

## Requirements

- Rust 1.70+
- Node.js 16+ (for frontend build)
- Tauri CLI
- Your preferred Notion API key

## Installation

### From Source

1. Clone the repository:
```bash
git clone https://github.com/unknowndevice077/notion-ai-assistant.git
cd notion-ai-assistant
```

2. Install Tauri CLI (if not already installed):
```bash
cargo install tauri-cli
```

3. Install dependencies:
```bash
cd src-tauri
cargo build
```

4. Install frontend dependencies:
```bash
npm install
```

5. Run in development mode:
```bash
npm run tauri dev
```

6. Build for production:
```bash
npm run tauri build
```

## Configuration

All credentials are entered directly in the app, not through a `.env` file — there isn't one. Open the app, go to **Settings**, and:

- **Notion**: paste your integration token under the Notion section.
- **AI Agent**: either pick a local Ollama model (no key needed), or choose "My own API key" and paste an API key for your provider.

Both are stored securely via the OS keyring (Keychain on macOS, Credential Manager on Windows, Secret Service on Linux) — never written to a plaintext file.

### Notion Integration

1. Create an integration at https://www.notion.com/my-integrations
2. Copy the integration token
3. Paste it into the app's Settings tab
4. Grant the integration access to your Notion workspace

## Usage

### Starting the Assistant

```bash
npm run tauri dev
```

The assistant window will appear as a small, always-available overlay.

### Keyboard Shortcuts

- `Ctrl+Shift+A` (Windows/Linux) or `Cmd+Shift+A` (macOS) - Toggle assistant window
- `Escape` - Close assistant window
- `Enter` - Submit prompt

## Project Structure

```
├── src/
│   ├── lib.rs           # Rust backend library
│   └── main.rs          # Desktop app entry point
├── src-tauri/
│   ├── Cargo.toml       # Rust dependencies
│   └── ...
├── src-ui/              # Frontend source (if applicable)
├── package.json         # Node dependencies
└── tauri.conf.json      # Tauri configuration
```

## Architecture

### Backend (Rust)
- Handles Notion API communication
- Manages AI request processing
- Stores credentials securely
- Manages local SQLite database

### Frontend (Tauri)
- Provides the UI interface
- Communicates with Rust backend via IPC
- Displays AI responses
- Manages settings and preferences

## Building for Distribution

### Windows
```bash
npm run tauri build -- --target x86_64-pc-windows-gnu
```

### macOS
```bash
npm run tauri build
```

### Linux
```bash
npm run tauri build
```

## Security Considerations

- Credentials are stored using the system keyring (Keychain on macOS, Credential Manager on Windows, pass on Linux)
- All API communications use HTTPS
- No data is sent to external servers except to Notion and your AI provider
- Self-hosted deployment means full control over your data

## Performance

- Lightweight Rust backend with minimal resource usage
- Efficient async I/O with Tokio
- SQLite local caching for faster responses
- Optimized Tauri window with minimal memory footprint

## Troubleshooting

### Build Issues

If you encounter build issues:

1. Ensure Rust is up to date: `rustup update`
2. Clear cargo cache: `cargo clean`
3. Rebuild: `npm run tauri build`

### Runtime Issues

If the app crashes:

1. Check the console logs
2. Verify your API keys are correctly set
3. Ensure Notion integration has proper permissions

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## Support

For issues and questions, please open an issue on the GitHub repository.