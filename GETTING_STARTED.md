# Getting Started (Beginner Guide)

This guide walks you through getting Agent Governance Console (AGC) running on your computer, step by step, assuming no prior experience with Rust, the terminal, or GitHub.

AGC is a command-line tool (no graphical interface). Everything happens by typing commands into a terminal window.

---

## Windows

### 1. Open a terminal

Right-click the **Start** button and choose **Terminal** (or **Windows PowerShell** on older versions of Windows).

### 2. Check if Rust is installed

Type the following commands, pressing Enter after each one:

```powershell
rustc --version
cargo --version
```

- If you see version numbers (e.g. `rustc 1.78.0`), Rust is installed; skip to step 3.
- If you see an error like `'rustc' is not recognized as an internal or external command`, Rust is either not installed or not in your system's PATH.

Install Rust from **https://rustup.rs**: download and run `rustup-init.exe`, then follow the on-screen instructions (the default options are fine). After installation finishes, **close and reopen your terminal**, then repeat the version checks above.

### 3. Get the code (no Git knowledge needed)

1. Go to the repository page: https://github.com/9t29zhmwdh-coder/agent-governance-console
2. Click the green **Code** button.
3. Click **Download ZIP**.
4. Extract the ZIP file somewhere convenient, e.g. `Documents\agent-governance-console`.

If you already have Git installed and prefer it, you can instead run:

```powershell
git clone https://github.com/9t29zhmwdh-coder/agent-governance-console.git
```

### 4. Build the project

In your terminal, navigate into the extracted/cloned folder, then run:

```powershell
cargo build --release
```

This downloads dependencies and compiles the project. It can take a few minutes the first time.

### 5. Run it

Try the command-line tool first; it needs no setup, no credentials, and no running server:

```powershell
.\target\release\agc-cli.exe
```

**What to expect:** it prints the AGC version, the default API bind address, telemetry status and audit export path, initializes the trace/audit/policy subsystems, and finishes with a hint to run `agc-api` to start the REST API.

If you want to try the actual REST API server:

```powershell
.\target\release\agc-api.exe
```

Then, in a **second** terminal window, check that it responds:

```powershell
curl http://127.0.0.1:8080/health
```

Press `Ctrl-C` in the first terminal to stop the server. Everything AGC stores is in memory only, so nothing is left behind on disk (aside from the `target\` build folder).

<!-- TODO: Screenshot -->

### Troubleshooting

| Problem | Cause | Fix |
|---|---|---|
| `rustc`/`cargo` not recognized, even after installing | Terminal was opened before Rust finished installing | Close and reopen the terminal window (or restart your computer) so the updated PATH takes effect |
| `cargo build --release` fails with linker errors (e.g. `link.exe not found`) | Missing C++ Build Tools, required by Rust on Windows | Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the "Desktop development with C++" workload, then retry |
| `curl` command not found | Older Windows versions may lack `curl` | Use PowerShell's `Invoke-WebRequest http://127.0.0.1:8080/health` instead, or open the URL in a browser |

---

## Linux

### 1. Open a terminal

This depends on your desktop environment (GNOME, KDE, XFCE, etc.). Look for an application called **Terminal**, **Konsole**, or similar in your application menu; searching for "Terminal" usually finds it. Many distributions also support the keyboard shortcut `Ctrl+Alt+T`.

### 2. Check if Rust is installed

```bash
rustc --version
cargo --version
```

- If you see version numbers, Rust is installed; skip to step 3.
- If you see `command not found: rustc`, Rust is not installed or not in your PATH.

Install it using the official installer from **https://rustup.rs** by running the curl one-liner shown on that page:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Follow the prompts (the default installation option is fine). Afterwards, close and reopen your terminal (or run `source "$HOME/.cargo/env"`), then repeat the version checks.

### 3. Get the code (no Git knowledge needed)

1. Go to the repository page: https://github.com/9t29zhmwdh-coder/agent-governance-console
2. Click the green **Code** button.
3. Click **Download ZIP**.
4. Extract the ZIP file (most file managers let you right-click → "Extract Here").

If you have Git installed and prefer it:

```bash
git clone https://github.com/9t29zhmwdh-coder/agent-governance-console.git
```

### 4. Build the project

Navigate into the extracted/cloned folder in your terminal, then run:

```bash
cargo build --release
```

### 5. Run it

Try the command-line tool first; it needs no setup, no credentials, and no running server:

```bash
./target/release/agc-cli
```

**What to expect:** it prints the AGC version, the default API bind address, telemetry status and audit export path, initializes the trace/audit/policy subsystems, and finishes with a hint to run `agc-api` to start the REST API.

If you want to try the actual REST API server:

```bash
./target/release/agc-api
```

Then, in a **second** terminal window, check that it responds:

```bash
curl http://127.0.0.1:8080/health
```

Press `Ctrl-C` in the first terminal to stop the server. Everything AGC stores is in memory only, so nothing is left behind on disk (aside from the `target/` build folder).

### Troubleshooting

| Problem | Cause | Fix |
|---|---|---|
| `rustc`/`cargo` not found, even after installing | Terminal session still has the old PATH | Close and reopen the terminal, or run `source "$HOME/.cargo/env"` |
| `cargo build --release` fails with a linker error (e.g. `error: linker 'cc' not found`) | No C compiler/build essentials installed | Install your distro's build tools, e.g. `sudo apt install build-essential` (Debian/Ubuntu) or the equivalent for your distribution |
| `curl: (7) Failed to connect` when checking `/health` | The `agc-api` server isn't running, or something else is using port 8080 | Make sure the server is still running in the other terminal; if the port is taken, stop the other process or check for typos in the URL |

---

## macOS

### 1. Open a terminal

Press `Cmd+Space` to open Spotlight, type **Terminal**, and press Enter.

### 2. Check if Rust is installed

```bash
rustc --version
cargo --version
```

- If you see version numbers, Rust is installed; skip to step 3.
- If you see `command not found: rustc`, Rust is not installed or not in your PATH.

Install it using the official installer from **https://rustup.rs** by running the curl one-liner shown on that page:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Follow the prompts (the default installation option is fine). Afterwards, close and reopen your terminal (or run `source "$HOME/.cargo/env"`), then repeat the version checks.

### 3. Get the code (no Git knowledge needed)

1. Go to the repository page: https://github.com/9t29zhmwdh-coder/agent-governance-console
2. Click the green **Code** button.
3. Click **Download ZIP**.
4. Double-click the downloaded ZIP file in Finder to extract it.

If you have Git installed and prefer it:

```bash
git clone https://github.com/9t29zhmwdh-coder/agent-governance-console.git
```

### 4. Build the project

Navigate into the extracted/cloned folder in your terminal, then run:

```bash
cargo build --release
```

### 5. Run it

Try the command-line tool first; it needs no setup, no credentials, and no running server:

```bash
./target/release/agc-cli
```

**What to expect:** it prints the AGC version, the default API bind address, telemetry status and audit export path, initializes the trace/audit/policy subsystems, and finishes with a hint to run `agc-api` to start the REST API.

If you want to try the actual REST API server:

```bash
./target/release/agc-api
```

Then, in a **second** terminal window, check that it responds:

```bash
curl http://127.0.0.1:8080/health
```

Press `Ctrl-C` in the first terminal to stop the server. Everything AGC stores is in memory only, so nothing is left behind on disk (aside from the `target/` build folder).

### Troubleshooting

| Problem | Cause | Fix |
|---|---|---|
| `rustc`/`cargo` not found, even after installing | Terminal session still has the old PATH | Close and reopen the terminal, or run `source "$HOME/.cargo/env"` |
| `cargo build --release` fails complaining about missing command line tools | Xcode Command Line Tools not installed | Run `xcode-select --install` and follow the prompts, then retry the build |
| `curl: (7) Failed to connect` when checking `/health` | The `agc-api` server isn't running, or something else is using port 8080 | Make sure the server is still running in the other terminal; if the port is taken, stop the other process or check for typos in the URL |

---

For a technical overview of features and API status, see the main [README.md](README.md). For architecture details, see [ARCHITECTURE.md](ARCHITECTURE.md).
