# Bijoy52 Cross-Platform System-Wide Typer — Plan

Goal: one open-source, lightweight app — not an AutoHotkey script, since AHK is
Windows-only — that installs on **Windows, Linux, and macOS** and lets you
toggle a global hotkey to type Bijoy52 Bangla in any app on that OS, the same
way `bijoy52-systemwide.ahk` already does on Windows alone.

**Status: built, tested, and verified working on Windows tonight.** See
`README.md` for usage/build instructions and what's confirmed vs. not. The
rest of this file is the original plan, kept for the reasoning behind the
decisions — one correction: the stack below was originally proposed as
Python, but after reviewing the cross-OS comparison table you chose **Rust +
rdev** instead (smaller/faster binary, ~1.5 MB release build), which is what
actually got built.

## Stack decision: Rust + rdev + enigo (originally proposed as Python)

You asked me to pick based on what genuinely fits all three operating
systems, so here's the reasoning, not just the conclusion.

The two things any candidate stack must do on **Windows, Linux (X11), and
macOS** are: (1) hook keyboard events globally, and (2) inject Unicode text
back. Four realistic options and how each one actually holds up per OS:

| Stack | Windows | Linux (X11) | macOS | Notes |
| --- | --- | --- | --- | --- |
| **Python + pynput** | ✅ `SetWindowsHookEx` | ✅ Xlib record extension | ✅ Quartz Event Tap (needs Accessibility permission grant) | Same library, same API, across all three — no per-OS branching in app logic |
| Rust + rdev | ✅ | ✅ | ✅ | Same coverage, smaller/faster binary, but harder for future contributors to read/audit than Python — this project's whole ethos so far (AHK script, plugin `main.js`) has been "plain text, read every line" |
| Node.js + uiohook-node | ✅ | ✅ | ✅ | Same coverage as pynput, but a packaged Node/Electron-ish app is meaningfully heavier ("lighter version" was explicitly requested) |
| Go + robotgo | ✅ | ✅ | ✅ | Viable middle ground, but the CGO dependency (robotgo wraps native libs) makes cross-compiling for all 3 OSes from one machine harder than Python or Rust |

**pynput wins on the actual constraint (equal, proven support on all three
OSes) while staying closest in spirit to the AHK script** — one readable
script, no build toolchain to understand, easy for anyone to open and verify
it isn't doing anything shady (same "Security notes" pattern already used in
`PLAN.md` and `Obsidian-Plugin-Plan.md`). `pystray` (tray icon) and
`PyInstaller` (turns the script into a standalone `.exe` / Linux binary / `.app`
so end users don't need Python installed) are the natural companions — both
are the standard, widely-used choice for exactly this.

**The one honest caveat, and it applies to every stack in that table, not
just this choice:** on **Linux with Wayland** (the default on current
Ubuntu, Fedora, etc.), no global keyboard hook or synthetic text injection
works from a regular user-space app — Wayland's security model blocks it by
design, the same restriction Avro Keyboard and every other system-wide IME
hits. The realistic answer is documented in "Known limitations" below:
XWayland-run apps are unaffected (X11 hook still reaches them), and native
Wayland apps are a documented gap, not a bug to chase in v1.

## Reusing the rule data instead of retranscribing it a third time

I checked `main.js` — the ~3,880 Bijoy52 rules live as a single JS object
literal (`{"key-sequence": "বাংলা", ...}`), which is already valid JSON. Instead
of hand-porting rules a third time (browser → AHK → Python, each transcription
being a fresh chance for typos), the plan is:

1. Mechanically extract that object literal from `main.js` (or the equivalent
   block in `bijoy52-systemwide.ahk`) into `rules.json`.
2. Have both this new app **and**, ideally, the existing `main.js` load from
   that same `rules.json` going forward, so there's one authoritative rule
   file instead of three copies that can silently drift apart.
3. Python engine embeds/loads `rules.json` and runs the identical
   longest-match-with-backtracking algorithm already proven in the other two
   versions (matra reordering, reph, conjuncts, lone-hasanta →
   independent-vowel).

This is a judgment call worth flagging: step 2 touches the existing
`main.js`, which your global rules say is hands-off without an explicit ask.
I'd only do step 1 (extract a `rules.json` for the new app) unless you
confirm you also want `main.js` refactored to load from it.

## Proposed folder layout

```
Bijoy52 Typer/
  System-Wide App/
    PLAN.md              <- this file
    rules.json            <- extracted Bijoy52 rule table (shared source of truth)
    bijoy52_engine.py      <- longest-match engine (same algorithm as the other two versions)
    bijoy52_app.py          <- hotkey listener, tray icon, toggle state, text injection
    requirements.txt         <- pynput, pystray, Pillow (tray icon rendering)
    tests/
      test_engine.py          <- round-trip test over all rules.json entries, mirroring
                                 the 3,881/3,881 check already done for index.html
    build/
      build_windows.md         <- PyInstaller command + notes for a .exe
      build_linux.md            <- PyInstaller command + notes for a binary
      build_macos.md             <- PyInstaller command + Accessibility-permission notes
    .github/workflows/
      build-release.yml          <- CI matrix building all 3 platform binaries on tag push
    README.md
    LICENSE                       <- MIT, matching the rest of the project
```

## Hotkey design

- Default toggle: **`Ctrl+Alt+B`** (not CapsLock like the AHK version) —
  CapsLock's OS-level "toggle" behavior varies enough between Windows/Linux/
  macOS that a modifier-combo hotkey is the more reliable default across all
  three. CapsLock stays available as a documented one-line config change for
  anyone who prefers it, exactly like the AHK script already does.
- On toggle: small OS-native notification/tray tooltip confirming
  **বাংলা (Bijoy52)** vs **EN**, mirroring the AHK version's tray behavior.
- Engine only intercepts the actual Bijoy52 key set while Bangla mode is on;
  everything else (modifier combos, function keys, unrelated punctuation)
  passes through untouched — same design already validated in both existing
  versions.

## Per-OS permission notes (to document in README, not solvable in code)

- **Windows:** no special permission needed, same as the AHK version.
- **macOS:** the app must be granted **Accessibility** access (System
  Settings → Privacy & Security → Accessibility) before the key hook works
  at all — this is a one-time manual grant, standard for any keystroke tool
  on macOS (same requirement Karabiner, Rectangle, etc. all have).
- **Linux (X11):** works out of the box under most distros; some setups
  restrict raw input access to the `input` group — documented as a one-line
  `usermod -aG input $USER` fix if needed.
- **Linux (Wayland):** documented known limitation, see above.

## Testing plan before calling this done

1. `tests/test_engine.py` — round-trip every entry in `rules.json` through
   the Python engine, same bar as the 3,881/3,881 pass already achieved for
   `index.html`. This part is fully verifiable in this environment, no OS
   install needed.
2. Real smoke test (needs the actual target machine, same honesty-check
   pattern as the AHK and Obsidian plans): toggle hotkey, `dj` → কি, `gf` →
   আ, `ufbd` → জানি, Backspace mid-sequence, switching focus mid-word.
   I cannot run this myself in this environment — you'd run it on Windows,
   Linux, and macOS separately and report back what happened.

## Open-source packaging

- **License:** MIT, consistent with the Obsidian plugin.
- **README:** install instructions per OS (download prebuilt binary from
  Releases vs. `pip install -r requirements.txt && python bijoy52_app.py`),
  hotkey config, and the same plain-language "Security notes" section
  pattern already used in the other two plan docs (no network calls, no
  telemetry, reads keys only while Bangla mode is on).
- **CI:** a GitHub Actions matrix build (windows-latest / ubuntu-latest /
  macos-latest) that runs PyInstaller on tag push and attaches all three
  binaries to a GitHub Release automatically — so "open source app for
  Windows, Linux, and Mac" means one tag push produces installable binaries
  for all three, not three separate manual builds.

## Known limitations (carried over / new)

- Native Wayland: not supported in v1 (see above) — XWayland apps unaffected.
- Same physical-US-QWERTY assumption for punctuation keys as the AHK/browser
  versions.
- Apps with anti-injection protection (elevated/admin windows on Windows,
  sandboxed apps on macOS) are unreachable by any keyboard hook, by OS
  design — identical limitation the AHK script already documents.

## What actually happened (resolved)

1. `main.js`/the Obsidian plugin were **not** touched — only new files under
   `System-Wide App/`.
2. CI (`.github/workflows/build-release.yml`) — not added yet; holding off
   until this is actually pushed to a real GitHub repo, since publishing
   both the plugin and this app tonight is the current priority.
3. Default hotkey shipped as `Ctrl+Alt+B`, confirmed working live.
4. Environment gaps hit and fixed along the way: no Rust toolchain (installed
   via `rustup-init` with the GNU host, avoiding a multi-GB Visual Studio
   install), and no `dlltool.exe` for the `windows` crate's GNU-target build
   (installed via winget's WinLibs MinGW-w64 package).
5. A real bug surfaced and got fixed: calling `enigo.text()` synchronously
   inside the `rdev::grab` callback silently failed to inject anything on
   Windows (SendInput re-entering the same low-level hook chain). Fixed by
   moving injection to a dedicated thread via a channel — see README's "How
   it works".
