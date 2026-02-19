# 🦀 Ferris Focus

A lightweight, gamified Pomodoro timer built in Rust with Iced.

## Features

- **Pomodoro Timer** — 25/5/15 min focus/break cycles with a circular progress ring
- **Focus Streaks & XP** — earn XP for completing sessions, build daily streaks, level up Ferris
- **Ferris Evolution** — watch Ferris grow: 🥚 → 🐣 → 🦀 → ⭐ → 👑
- **Stats Dashboard** — daily/all-time stats, weekly session heatmap
- **Desktop Notifications** — alerts when sessions complete
- **Persistent** — SQLite storage, your progress survives restarts

## Install

### From binary (easiest)

Download the latest binary for your platform from [Releases](https://github.com/sakshyam-sh/ferris-focus/releases):

| Platform | File |
|---|---|
| Linux | `ferris-focus-linux` |
| macOS (Apple Silicon) | `ferris-focus-macos` |
| Windows | `ferris-focus.exe` |
| Debian/Ubuntu | `ferris-focus_0.1.0-1_amd64.deb` |

**Linux / macOS:**
```bash
chmod +x ferris-focus-linux   # or ferris-focus-macos
./ferris-focus-linux
```

**Windows:** just double-click `ferris-focus.exe`.

**Debian/Ubuntu:**
```bash
sudo dpkg -i ferris-focus_0.1.0-1_amd64.deb
ferris-focus
```

### From source

```bash
git clone https://github.com/sakshyam-sh/ferris-focus.git
cd ferris-focus
cargo install --path .
ferris-focus
```

## Build

```bash
cargo build --release
./target/release/ferris-focus
```

## Tech Stack

- **GUI**: [Iced](https://iced.rs) 0.14
- **DB**: SQLite via rusqlite
- **Notifications**: notify-rust
- **Language**: Rust 🦀

## License

MIT
