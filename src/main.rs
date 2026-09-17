// Bijoy52 system-wide typer — cross-platform (Windows/Linux/macOS) port of
// the same engine used by the Obsidian plugin and the AutoHotkey script.
// Global key hook via `rdev::grab` (observe + suppress), Unicode text
// injection via `enigo` (rdev's own `simulate` can't send arbitrary Unicode,
// only named physical keys).
//
// No network calls, no telemetry. Only acts on keys while Bangla mode is on.

mod engine;

use engine::BijoyEngine;
use enigo::{Enigo, Keyboard, Settings};
use rdev::{grab, Event, EventType, Key};

const TOGGLE_KEY: Key = Key::KeyB; // Ctrl+Alt+B toggles Bangla mode

#[derive(Default)]
struct Modifiers {
    ctrl: bool,
    alt: bool,
    meta: bool,
}

impl Modifiers {
    fn any(&self) -> bool {
        self.ctrl || self.alt || self.meta
    }
}

fn is_nav_key(key: Key) -> bool {
    matches!(
        key,
        Key::LeftArrow
            | Key::RightArrow
            | Key::UpArrow
            | Key::DownArrow
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown
    )
}

struct State {
    engine: BijoyEngine,
    bangla_mode: bool,
    mods: Modifiers,
}

fn main() {
    // Text injection must NOT happen synchronously inside the low-level
    // keyboard hook callback: on Windows, calling SendInput from within a
    // WH_KEYBOARD_LL hook re-enters the same hook chain before the OS has
    // finished processing the original event, which unreliably drops or
    // misorders the injected keystrokes. So the hook thread only decides
    // *what* to inject and hands the string off through a channel; a
    // separate thread owns `Enigo` and actually calls `.text()`.
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        let mut enigo = Enigo::new(&Settings::default()).expect(
            "failed to initialize input simulator (this needs a normal desktop session)",
        );
        for text in rx {
            if let Err(e) = enigo.text(&text) {
                eprintln!("Bijoy52: failed to inject text {text:?}: {e:?}");
            }
        }
    });

    // `rdev::grab` requires `Fn`, not `FnMut` — the hook callback is invoked
    // synchronously from the single thread that calls `grab()`, so a RefCell
    // is enough for the mutable state shared across calls (no cross-thread
    // access, so no Mutex needed).
    let state = std::cell::RefCell::new(State {
        engine: BijoyEngine::new(),
        bangla_mode: false,
        mods: Modifiers::default(),
    });

    println!("Bijoy52 Typer running.");
    println!("Press Ctrl+Alt+B to toggle বাংলা (Bijoy52) mode.");
    println!("Close this window, or Ctrl+C in it, to quit.");

    let callback = move |event: Event| -> Option<Event> {
        let mut guard = state.borrow_mut();
        let State {
            engine,
            bangla_mode,
            mods,
        } = &mut *guard;

        match event.event_type {
            EventType::KeyPress(key) => {
                match key {
                    Key::ControlLeft | Key::ControlRight => {
                        mods.ctrl = true;
                        return Some(event);
                    }
                    Key::Alt | Key::AltGr => {
                        mods.alt = true;
                        return Some(event);
                    }
                    Key::MetaLeft | Key::MetaRight => {
                        mods.meta = true;
                        return Some(event);
                    }
                    k if k == TOGGLE_KEY && mods.ctrl && mods.alt => {
                        *bangla_mode = !*bangla_mode;
                        engine.reset_partial();
                        println!(
                            "{}",
                            if *bangla_mode {
                                "বাংলা (Bijoy52) mode: ON"
                            } else {
                                "English mode: OFF"
                            }
                        );
                        return None; // swallow the toggle keystroke itself
                    }
                    _ => {}
                }

                if !*bangla_mode {
                    return Some(event);
                }
                if mods.any() {
                    return Some(event); // don't hijack Ctrl/Alt/Cmd shortcuts
                }

                if is_nav_key(key) {
                    engine.reset_partial();
                    return Some(event);
                }

                match key {
                    Key::Backspace => {
                        if engine.backspace() {
                            return None; // mid-sequence undo: nothing was ever shown
                        }
                        engine.reset_partial();
                        return Some(event);
                    }
                    Key::Return | Key::KpReturn | Key::Tab | Key::Escape => {
                        engine.reset_partial();
                        return Some(event);
                    }
                    _ => {}
                }

                let ch = match event.name.as_deref() {
                    Some(s) if s.chars().count() == 1 => s.chars().next().unwrap(),
                    _ => return Some(event), // multi-char/dead-key/unknown: leave untouched
                };

                let replacement = engine.feed(ch);
                if !replacement.is_empty() {
                    let _ = tx.send(replacement);
                }
                None // suppress the physical key; the injector thread sends the mapped text
            }
            EventType::KeyRelease(key) => {
                match key {
                    Key::ControlLeft | Key::ControlRight => mods.ctrl = false,
                    Key::Alt | Key::AltGr => mods.alt = false,
                    Key::MetaLeft | Key::MetaRight => mods.meta = false,
                    _ => {}
                }
                Some(event)
            }
            _ => Some(event),
        }
    };

    if let Err(err) = grab(callback) {
        eprintln!("Failed to start global key grab: {err:?}");
        eprintln!(
            "On macOS: grant this app Accessibility access in System Settings and re-run."
        );
        eprintln!(
            "On Linux: make sure your user is in the `input` group, or run with sudo."
        );
        std::process::exit(1);
    }
}
