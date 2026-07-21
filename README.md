<p align="center"><strong>Codex CLI</strong> is a coding agent from OpenAI that runs locally on your computer.
<p align="center">
  <img src="https://github.com/openai/codex/blob/main/.github/codex-cli-splash.png" alt="Codex CLI splash" width="80%" />
</p>
</br>
If you want Codex in your code editor (VS Code, Cursor, Windsurf), <a href="https://developers.openai.com/codex/ide">install in your IDE.</a>
</br>If you want the desktop app experience, run <code>codex app</code> or visit <a href="https://chatgpt.com/codex?app-landing-page=true">the Codex App page</a>.
</br>If you are looking for the <em>cloud-based agent</em> from OpenAI, <strong>Codex Web</strong>, go to <a href="https://chatgpt.com/codex">chatgpt.com/codex</a>.</p>

---

## TAMU AI Chat fork

This fork keeps Codex's agent loop and function tools while using TAMU AI Chat. Release archives are built natively for Linux, macOS, and Windows. Linux releases use static musl linking so they do not depend on the host's glibc or OpenSSL version.

### Release files

Download files from [GitHub Releases](https://github.com/fhwutamu/codex-tamuaichat/releases). Each release includes `SHA256SUMS` and these archives:

| Platform | Architecture | Release file |
| --- | --- | --- |
| Linux | x86-64 | `codex-x86_64-unknown-linux-musl.tar.gz` |
| Linux | ARM64 | `codex-aarch64-unknown-linux-musl.tar.gz` |
| macOS | Apple Silicon | `codex-aarch64-apple-darwin.tar.gz` |
| macOS | Intel | `codex-x86_64-apple-darwin.tar.gz` |
| Windows | x86-64 | `codex-x86_64-pc-windows-msvc.zip` |
| Windows | ARM64 | `codex-aarch64-pc-windows-msvc.zip` |

The examples below install `v0.1.1`. Change `VERSION` when installing a later release.

### Install on Linux

```shell
VERSION=v0.1.1
case "$(uname -m)" in
  x86_64) TARGET=x86_64-unknown-linux-musl ;;
  aarch64|arm64) TARGET=aarch64-unknown-linux-musl ;;
  *) echo "Unsupported Linux architecture: $(uname -m)" >&2; exit 1 ;;
esac

BASE_URL="https://github.com/fhwutamu/codex-tamuaichat/releases/download/${VERSION}"
curl -fL "${BASE_URL}/codex-${TARGET}.tar.gz" -o "/tmp/codex-${TARGET}.tar.gz"
curl -fL "${BASE_URL}/SHA256SUMS" -o /tmp/codex-SHA256SUMS
(cd /tmp && grep "codex-${TARGET}.tar.gz" codex-SHA256SUMS | sha256sum -c -)

INSTALL_DIR="$HOME/.local/lib/codex-tamu"
mkdir -p "$INSTALL_DIR" "$HOME/.local/bin"
tar -xzf "/tmp/codex-${TARGET}.tar.gz" -C "$INSTALL_DIR"
ln -sfn "$INSTALL_DIR/codex" "$HOME/.local/bin/codex-tamu"
export PATH="$HOME/.local/bin:$PATH"
codex-tamu --version
```

The Linux archive also contains `codex-resources/bwrap`, which Codex uses when the host does not provide a suitable Bubblewrap executable. Add `$HOME/.local/bin` to `PATH` if `codex-tamu` is not found.

### Install on macOS

```shell
VERSION=v0.1.1
case "$(uname -m)" in
  arm64) TARGET=aarch64-apple-darwin ;;
  x86_64) TARGET=x86_64-apple-darwin ;;
  *) echo "Unsupported macOS architecture: $(uname -m)" >&2; exit 1 ;;
esac

BASE_URL="https://github.com/fhwutamu/codex-tamuaichat/releases/download/${VERSION}"
curl -fL "${BASE_URL}/codex-${TARGET}.tar.gz" -o "/tmp/codex-${TARGET}.tar.gz"
curl -fL "${BASE_URL}/SHA256SUMS" -o /tmp/codex-SHA256SUMS
(cd /tmp && grep "codex-${TARGET}.tar.gz" codex-SHA256SUMS | shasum -a 256 -c -)

INSTALL_DIR="$HOME/.local/lib/codex-tamu"
mkdir -p "$INSTALL_DIR" "$HOME/.local/bin"
tar -xzf "/tmp/codex-${TARGET}.tar.gz" -C "$INSTALL_DIR"
xattr -d com.apple.quarantine "$INSTALL_DIR/codex" 2>/dev/null || true
ln -sfn "$INSTALL_DIR/codex" "$HOME/.local/bin/codex-tamu"
export PATH="$HOME/.local/bin:$PATH"
codex-tamu --version
```

The macOS binaries are ad-hoc signed but not Apple-notarized. Removing the quarantine attribute is therefore required after downloading them from a browser or GitHub.

### Install on Windows

Run the following in PowerShell:

```powershell
$Version = "v0.1.1"
$Architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
$Target = if ($Architecture -eq "Arm64") {
    "aarch64-pc-windows-msvc"
} elseif ($Architecture -eq "X64") {
    "x86_64-pc-windows-msvc"
} else {
    throw "Unsupported Windows architecture: $Architecture"
}

$BaseUrl = "https://github.com/fhwutamu/codex-tamuaichat/releases/download/$Version"
$Archive = Join-Path $env:TEMP "codex-$Target.zip"
$Checksums = Join-Path $env:TEMP "codex-SHA256SUMS"
Invoke-WebRequest "$BaseUrl/codex-$Target.zip" -OutFile $Archive
Invoke-WebRequest "$BaseUrl/SHA256SUMS" -OutFile $Checksums

$Expected = ((Select-String "codex-$Target.zip" $Checksums).Line -split '\s+')[0].ToLower()
$Actual = (Get-FileHash $Archive -Algorithm SHA256).Hash.ToLower()
if ($Actual -ne $Expected) {
    throw "SHA256 mismatch: expected $Expected, got $Actual"
}

$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\codex-tamu"
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Expand-Archive $Archive -DestinationPath $InstallDir -Force

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (($UserPath -split ';') -notcontains $InstallDir) {
    [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$UserPath", "User")
}
$env:Path = "$InstallDir;$env:Path"
codex.exe --version
```

The Windows ZIP includes `codex-command-runner.exe` and `codex-windows-sandbox-setup.exe`; keep all three executables in the same directory.

### Configure TAMU AI Chat

The provider reads its API key from `TAMUS_AI_CHAT_API_KEY`. For the current Bash or Zsh session:

```shell
read -rsp "TAMU AI Chat API key: " TAMUS_AI_CHAT_API_KEY
echo
export TAMUS_AI_CHAT_API_KEY
```

For the current PowerShell session:

```powershell
$env:TAMUS_AI_CHAT_API_KEY = Read-Host "TAMU AI Chat API key"
```

Create or update `~/.codex/config.toml` on Linux/macOS or `$HOME\.codex\config.toml` on Windows:

```toml
model_provider = "tamu-ai-chat"
model = "gpt-5.4"
```

Start the interactive agent from a project directory:

```shell
codex-tamu
```

On Windows, run `codex.exe`. Use `/model` inside the TUI to choose any model labeled `via TAMU AI Chat`; do not add the `protected.` prefix yourself.

For a one-off model selection without changing the config file:

```shell
codex-tamu -c 'model_provider="tamu-ai-chat"' -m "Claude Sonnet 4.6"
```

The provider calls TAMU's Chat Completions endpoint, adds the required `protected.` model prefix, and supports function-calling models.
It does not expose Responses-only hosted, namespace, or freeform tools, structured output schemas, or Responses WebSockets.

### Build from source

```shell
cd codex-rs
cargo build --release -p codex-cli --bin codex
```

For distributable artifacts, use `.github/workflows/tamu-release.yml`; native release builds avoid relying on a developer workstation's system libraries.

## Upstream Codex quickstart

The commands below install the upstream OpenAI Codex distribution, not this TAMU AI Chat fork. Use the release instructions above when you need the built-in `tamu-ai-chat` provider.

### Installing and running Codex CLI

Run the following on Mac or Linux to install Codex CLI:

```shell
curl -fsSL https://chatgpt.com/codex/install.sh | sh
```

Run the following on Windows to install Codex CLI:

```shell
powershell -ExecutionPolicy ByPass -c "irm https://chatgpt.com/codex/install.ps1 | iex"
```

Codex CLI can also be installed via the following package managers:

```shell
# Install using npm
npm install -g @openai/codex
```

```shell
# Install using Homebrew
brew install --cask codex
```

Then simply run `codex` to get started.

<details>
<summary>You can also go to the <a href="https://github.com/openai/codex/releases/latest">latest GitHub Release</a> and download the appropriate binary for your platform.</summary>

Each GitHub Release contains many executables, but in practice, you likely want one of these:

- macOS
  - Apple Silicon/arm64: `codex-aarch64-apple-darwin.tar.gz`
  - x86_64 (older Mac hardware): `codex-x86_64-apple-darwin.tar.gz`
- Linux
  - x86_64: `codex-x86_64-unknown-linux-musl.tar.gz`
  - arm64: `codex-aarch64-unknown-linux-musl.tar.gz`

Each archive contains a single entry with the platform baked into the name (e.g., `codex-x86_64-unknown-linux-musl`), so you likely want to rename it to `codex` after extracting it.

</details>

### Using Codex with your ChatGPT plan

Run `codex` and select **Sign in with ChatGPT**. We recommend signing into your ChatGPT account to use Codex as part of your Plus, Pro, Business, Edu, or Enterprise plan. [Learn more about what's included in your ChatGPT plan](https://help.openai.com/en/articles/11369540-codex-in-chatgpt).

You can also use Codex with an API key, but this requires [additional setup](https://developers.openai.com/codex/auth#sign-in-with-an-api-key).

## Docs

- [**Codex Documentation**](https://developers.openai.com/codex)
- [**Contributing**](./docs/contributing.md)
- [**Installing & building**](./docs/install.md)
- [**Open source fund**](./docs/open-source-fund.md)

This repository is licensed under the [Apache-2.0 License](LICENSE).
