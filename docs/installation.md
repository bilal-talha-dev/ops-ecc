# Installation & Cross-Platform Setup Guide

`ops-ecc` is distributed in three ways:
1. **`crates.io`** (`cargo install ops-ecc`) — compiles the latest release from source using Rust.
2. **GitHub Releases** ([`v0.1.0` Assets](https://github.com/bilal-talha-dev/ops-ecc/releases/tag/v0.1.0)) — prebuilt, statically linked binaries for **Windows**, **macOS**, and **Linux** (zero Rust installation required).
3. **Optional `npm` / `npx` Wrapper** (`npm-wrapper/`) — thin launcher that downloads the matching GitHub Release binary.

---

## Method 1: Install via Cargo (`crates.io`)

### Step 1: Install Rust (`rustc >= 1.78`) if not already installed
- **macOS / Linux:**
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source "$HOME/.cargo/env"
  ```
- **Windows (PowerShell):**
  ```powershell
  Invoke-WebRequest -Uri "https://win.rustup.rs/" -OutFile "rustup-init.exe"
  .ustup-init.exe -y
  ```
  *(Restart PowerShell after installation.)*

### Step 2: Install `ops-ecc`
```bash
cargo install ops-ecc
```

Verify installation:
```bash
ops-ecc --version
ops-ecc doctor
```

---

## Method 2: Install Prebuilt Binary from GitHub Releases (No Rust Required)

Download the archive for your operating system from **[GitHub Releases (`v0.1.0`)](https://github.com/bilal-talha-dev/ops-ecc/releases/tag/v0.1.0)**:

| OS / Architecture | Download File | Executable Inside |
| :--- | :--- | :--- |
| **Windows (x64)** | `ops-ecc-windows-x86_64.zip` | `ops-ecc.exe` |
| **macOS (Apple Silicon M1/M2/M3/M4)** | `ops-ecc-macos-arm64.tar.gz` | `ops-ecc` |
| **macOS (Intel)** | `ops-ecc-macos-x86_64.tar.gz` | `ops-ecc` |
| **Linux / Ubuntu (x64)** | `ops-ecc-linux-x86_64.tar.gz` | `ops-ecc` |
| **Linux / Ubuntu (ARM64)** | `ops-ecc-linux-arm64.tar.gz` | `ops-ecc` |

### Windows Setup (PowerShell)
1. Download `ops-ecc-windows-x86_64.zip` from the Releases page.
2. Extract it to a permanent folder (e.g., `C:\tools\ops-ecc\`):
   ```powershell
   New-Item -ItemType Directory -Force -Path C:\tools\ops-ecc
   Expand-Archive -Path "$env:USERPROFILE\Downloads\ops-ecc-windows-x86_64.zip" -DestinationPath C:\tools\ops-ecc -Force
   ```
3. Add `C:\tools\ops-ecc` to your User `PATH` permanently:
   ```powershell
   [Environment]::SetEnvironmentVariable(
     "Path",
     "C:\tools\ops-ecc;" + [Environment]::GetEnvironmentVariable("Path", "User"),
     "User"
   )
   ```
4. Restart PowerShell and verify:
   ```powershell
   ops-ecc doctor
   ```

### macOS Setup (Zsh)
```zsh
cd ~/Downloads
tar -xzf ops-ecc-macos-arm64.tar.gz   # or ops-ecc-macos-x86_64.tar.gz for Intel
chmod +x ops-ecc
sudo mv ops-ecc /usr/local/bin/

# If macOS Gatekeeper flags the unsigned binary:
xattr -d com.apple.quarantine /usr/local/bin/ops-ecc 2>/dev/null || true

ops-ecc doctor
```

### Linux / Ubuntu Setup (Bash)
```bash
cd ~/Downloads
tar -xzf ops-ecc-linux-x86_64.tar.gz   # or ops-ecc-linux-arm64.tar.gz
chmod +x ops-ecc
sudo mv ops-ecc /usr/local/bin/
ops-ecc doctor
```

### Verifying Checksums (`.sha256`)
Every release asset includes a matching `.sha256` file:
```bash
# Linux / macOS
shasum -a 256 -c ops-ecc-linux-x86_64.tar.gz.sha256
```
```powershell
# Windows PowerShell
Get-FileHash ops-ecc-windows-x86_64.zip -Algorithm SHA256
```

---

## Method 3: Build from Source

```bash
git clone https://github.com/bilal-talha-dev/ops-ecc.git
cd ops-ecc
cargo test --all-targets
cargo build --release
cargo install --path .
```

---

## Shell Completions Setup

Generate native tab-completion scripts for your shell:

- **Bash (`~/.bashrc`)**:
  ```bash
  mkdir -p ~/.local/share/bash-completion/completions
  ops-ecc completions bash > ~/.local/share/bash-completion/completions/ops-ecc
  source ~/.bashrc
  ```
- **Zsh (`~/.zshrc`)**:
  ```zsh
  mkdir -p ~/.zfunc
  ops-ecc completions zsh > ~/.zfunc/_ops-ecc
  # Ensure fpath=(~/.zfunc $fpath) and compinit are in ~/.zshrc
  ```
- **PowerShell (`$PROFILE`)**:
  ```powershell
  ops-ecc completions powershell >> $PROFILE
  ```
