# COSMIC Process Killer

A native applet for the COSMIC Desktop Environment designed to manage and kill frozen or resource-heavy processes efficiently.

![License](https://img.shields.io/badge/license-MIT-blue.svg) ![Rust](https://img.shields.io/badge/built_with-Rust-orange.svg) ![COSMIC](https://img.shields.io/badge/desktop-COSMIC-purple.svg)

## 🚀 Features

- **Process Monitoring**: Real-time list of top resource-consuming processes.
- **Smart Filtering**: Shows top 10 CPU consumers by default, with a "Show All" toggle.
- **Search**: Quickly find processes by Name or PID.
- **Sorting**: Sort by Name, PID, CPU usage, or Memory usage.
- **Process Management**:
  - **Kill (SIGTERM)**: Gracefully request the process to stop.
  - **Force Kill (SIGKILL)**: Immediately terminate the process.
- **Standalone Mode**: A separate window mode that works even if the panel crashes.
- **Localization**: Full support for English (en) and Portuguese (pt-BR).

## 🛠️ Requirements

- **COSMIC Desktop Environment** (libcosmic)
- **Rust** (latest stable)
- **Just** (command runner)

## 📦 Installation

### Arch Linux

```bash
sudo pacman -S rust cargo git just
git clone https://github.com/marcossl10/cosmic-process-killer.git
cd cosmic-process-killer
cargo build --release
sudo just install
```

### Fedora Linux

```bash
sudo dnf install rust cargo git just
git clone https://github.com/marcossl10/cosmic-process-killer.git
cd cosmic-process-killer
cargo build --release
sudo just install
```

### Pop!_OS

```bash
sudo apt install rustc cargo git just
git clone https://github.com/marcossl10/cosmic-process-killer.git
cd cosmic-process-killer
cargo build --release
sudo just install
```

## 🎯 Usage

### Normal Mode (Applet in Panel)

1. After installation, add the applet to your COSMIC panel via Panel Settings.
2. Click the icon to view processes.
3. Click column headers (Name, CPU, Mem) to sort.
4. Use the search bar to filter specific applications.

### 🚨 Emergency Mode (Standalone)

**What if the panel freezes?** Don't worry! There's a standalone mode that works independently:

```bash
cosmic-process-killer
```

**Tip:** Configure a keyboard shortcut (like `Ctrl+Shift+Esc`) for `cosmic-process-killer` in COSMIC Settings → Keyboard → Shortcuts.

## ⚠️ Warnings

- **Be careful when killing processes**: Terminating system processes can cause instability.
- **Permissions**: You may need elevated privileges to kill some processes.
- **Force Kill**: Use only when normal termination doesn't work.

## 📄 License

Distributed under the MIT License. See `LICENSE` for more information.


