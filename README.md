# Bijoy52 Typer (system-wide, cross-platform)

Type Bijoy52 Bangla in **any** app on Windows, Linux, or macOS via a global
hotkey toggle — no OS-level keyboard layout install. Same longest-match
engine as the [Obsidian plugin](../Obsidian%20Plugin/bijoy52-typer/) and the
[AutoHotkey script](../bijoy52-systemwide.ahk) versions of this tool, ported
to Rust so it runs as one small (~1.5 MB), dependency-free binary on all
three OSes instead of only Windows.

## Usage

1. Download the binary from
   [Releases](https://github.com/harunrhimu/bijoy52-typer/releases) and run
   it (double-click it, or from a terminal). **Windows will show a
   "Windows protected your PC" SmartScreen warning** — this is expected for
   any new, unsigned executable from an "unknown publisher," not a sign
   something's wrong. Getting rid of it requires a paid code-signing
   certificate, which no solo/open-source first release has. Click **"More
   info" → "Run anyway"**. If you want to verify the download wasn't
   tampered with, check its SHA256 hash against the one listed on the
   release page.
2. Press **Ctrl+Alt+B** to toggle বাংলা (Bijoy52) mode on/off. The console
   prints which mode you're in.
3. Type Bijoy52 keystrokes in any app, exactly like the other versions of
   this tool.
4. Close the console window (or Ctrl+C in it) to quit.

## Building from source

Requires the Rust toolchain (https://rustup.rs/).

```
cargo build --release
```

The binary is produced at `target/release/bijoy52-typer` (`.exe` on
Windows).

On Windows without Visual Studio Build Tools installed, use the GNU-host
toolchain instead (`rustup-init --default-host x86_64-pc-windows-gnu`), plus
a MinGW-w64 distribution on `PATH` (needed for `dlltool.exe`, e.g.
[WinLibs](https://winlibs.com/)) — the MSVC toolchain isn't required.

## How it works

- **Global key hook & suppression:** [`rdev`](https://crates.io/crates/rdev)
  (`unstable_grab` feature) installs a low-level keyboard hook, observes
  every keystroke, and can swallow the original one (`return None`).
- **Unicode text injection:** [`enigo`](https://crates.io/crates/enigo)'s
  `Keyboard::text()`, since `rdev`'s own `simulate()` only sends named
  physical keys, not arbitrary Unicode. Injection runs on its own thread,
  not inside the hook callback — calling `SendInput` synchronously from
  within a Windows low-level keyboard hook is a documented reentrancy trap
  that drops/misorders the injected keystrokes.
- **Engine:** `src/engine.rs` is a direct port of the same longest-match,
  backtracking state machine used in the Obsidian plugin's `main.js`,
  loading the same `rules.json` (~3,880 Bijoy Classic → Unicode rules,
  extracted from the authoritative `bn-bijoyUnicode.mim` rule set, originally
  from Ananda Computers).
- Only intercepts keys while Bangla mode is on, and only the ~72 physical
  keys Bijoy52 actually uses — Ctrl/Alt/Cmd shortcuts, function keys, and
  navigation keys always pass through untouched.

## Verified vs. not

- `cargo test` round-trips all 3,881 rules through the engine and checks
  several full words (আমি, জানি, কি, আ) — passes.
- Live-tested on Windows: global hotkey toggle, `dj`→কি, a full sentence
  (`gfdm ufbd`→আমি জানি), mid-sequence Backspace-then-retype, and Ctrl+A/
  Ctrl+C passing through untouched — all confirmed working by driving real
  keystrokes into Notepad and reading back the result.
- **Not yet tested on Linux or macOS** — same algorithm and libraries, but
  needs a real run on each OS to confirm. Please report back what happened
  (exact keys pressed vs. what came out) if anything's off.

## Known limitations

- **Linux Wayland:** no global keyboard hook or synthetic text injection
  reaches native Wayland apps — this is a deliberate Wayland security
  restriction, not a bug here. XWayland-run apps are unaffected. Same
  limitation every system-wide IME (Avro Keyboard, etc.) hits.
- **macOS:** needs Accessibility permission granted once (System Settings →
  Privacy & Security → Accessibility) before the hook works at all.
- **Linux (X11):** some distros restrict raw input access to the `input`
  group (`sudo usermod -aG input $USER`, then log out/in).
- Assumes physical US QWERTY key positions for punctuation, same assumption
  Bijoy/Avro make.
- No system tray icon yet (console window only) — the toggle notification
  is a printed line, not a tray tooltip. Deferred to keep tonight's build
  scope realistic; a tray icon is a reasonable fast-follow.
- No auto-flush-on-pause timer (the Obsidian plugin has a 450ms one) — a
  half-typed sequence stays buffered (nothing shown) until the next
  keystroke resolves it. In normal typing this rarely matters since words
  are followed by a space or punctuation that resolves it anyway.

## Security notes

- No network calls, no telemetry, no external services.
- Only reacts to keys while Bangla mode is toggled on.
- Global key hooks are how every keyboard layout/IME works, including the
  OS's own — not a red flag by itself. Source is plain Rust, no obfuscation;
  read `src/main.rs` and `src/engine.rs` end to end.
