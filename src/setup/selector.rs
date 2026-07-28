#![forbid(unsafe_code)]

//! Interactive TUI selector for `mine setup`.
//!
//! Renders a checklist of detected coding agents:
//!   [✓] Claude Code
//!   [ ] Codex
//!   [—] Pi          (greyed out, undetected, not selectable)
//!
//! Keys: Up/Down move focus, Space toggles, Enter confirms. Undetected agents
//! are shown greyed and cannot be selected. Agents that already have MINE
//! installed start checked.
//!
//! When stdin is not a TTY (CI, `curl|sh` pipes), the selector is skipped and
//! [`resolve_plan`] falls back to "install into every detected agent".

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::style::{Color, SetForegroundColor};
use crossterm::terminal::{
    BeginSynchronizedUpdate, Clear, ClearType, EndSynchronizedUpdate, EnterAlternateScreen,
    LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{cursor, queue, style::Print};
use std::io::{Write, stdout};

use crate::domain::error::{MineError, MineResult};

use super::SetupPlan;
use super::agent_detect::Detection;

/// An entry in the selector list.
struct Entry {
    slug: String,
    display: String,
    detected: bool,
    checked: bool,
}

/// Resolves the install/uninstall plan.
///
/// - TTY: run the interactive selector.
/// - non-TTY + `yes`: install into every detected agent; uninstall nothing.
/// - non-TTY without `yes`: same as `yes` (the selector cannot run without a
///   TTY, so the safe default is "install into all detected").
pub fn resolve_plan(
    detections: &[Detection],
    version_note: &str,
    _yes: bool,
    env: &crate::agent_setup::targets::Env,
) -> MineResult<SetupPlan> {
    // Determine which agents already have MINE installed (start checked).
    let state = crate::agent_setup::managed_state::ManagedState::load(&env.config_root)
        .unwrap_or_else(|_| crate::agent_setup::managed_state::ManagedState::new());

    let entries: Vec<Entry> = detections
        .iter()
        .map(|d| {
            let already = state.record(&d.slug).is_some();
            Entry {
                slug: d.slug.clone(),
                display: d.display_name.clone(),
                detected: d.detected,
                checked: d.detected && already,
            }
        })
        .collect();

    if is_tty() {
        match run_selector(entries, detections, version_note)? {
            Some(final_entries) => {
                let mut install = Vec::new();
                let mut uninstall = Vec::new();
                for d in detections {
                    let e = final_entries.iter().find(|e| e.slug == d.slug).unwrap();
                    if d.detected {
                        if e.checked {
                            install.push(d.slug.clone());
                        } else if state.record(&d.slug).is_some() {
                            uninstall.push(d.slug.clone());
                        }
                    }
                }
                Ok(SetupPlan {
                    install,
                    uninstall,
                    cancelled: false,
                })
            }
            None => Ok(SetupPlan {
                install: Vec::new(),
                uninstall: Vec::new(),
                cancelled: true,
            }),
        }
    } else {
        // Non-TTY fallback: install into every detected agent.
        let install: Vec<String> = detections
            .iter()
            .filter(|d| d.detected)
            .map(|d| d.slug.clone())
            .collect();
        if install.is_empty() {
            eprintln!("No coding agents detected; nothing to install.");
        }
        Ok(SetupPlan {
            install,
            uninstall: Vec::new(),
            cancelled: false,
        })
    }
}

/// Returns true if stdin is a TTY (terminal).
fn is_tty() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal()
}

/// Runs the interactive selector. Returns the final entry states.
fn run_selector(
    mut entries: Vec<Entry>,
    detections: &[Detection],
    version_note: &str,
) -> MineResult<Option<Vec<Entry>>> {
    let mut focus: usize = 0;
    for (i, e) in entries.iter().enumerate() {
        if e.detected {
            focus = i;
            break;
        }
    }

    let mut out = stdout();
    enable_raw_mode().map_err(|e| MineError::ExternalDependency {
        detail: format!("TUI: enable_raw_mode failed: {e}"),
    })?;
    let _ = execute!(out, EnterAlternateScreen);
    let outcome = selector_loop(&mut entries, &mut focus, &mut out, detections, version_note);
    let _ = execute!(out, LeaveAlternateScreen);
    let _ = disable_raw_mode();
    let _ = out.flush();
    match outcome? {
        SelectorOutcome::Confirmed => Ok(Some(entries)),
        SelectorOutcome::Cancelled => Ok(None),
    }
}

enum SelectorOutcome {
    Confirmed,
    Cancelled,
}

fn selector_loop(
    entries: &mut [Entry],
    focus: &mut usize,
    out: &mut std::io::Stdout,
    detections: &[Detection],
    version_note: &str,
) -> MineResult<SelectorOutcome> {
    loop {
        // Wrap each frame in a synchronized update so the terminal applies
        // the clear+redraw atomically instead of flickering.
        let _ = queue!(out, BeginSynchronizedUpdate);
        render_frame(out, entries, *focus, detections, version_note)?;
        let _ = queue!(out, EndSynchronizedUpdate);
        out.flush().ok();

        if !event::poll(std::time::Duration::from_millis(500)).unwrap_or(false) {
            continue;
        }
        let ev = match event::read() {
            Ok(e) => e,
            Err(e) => {
                return Err(MineError::ExternalDependency {
                    detail: format!("TUI: event read failed: {e}"),
                });
            }
        };
        if let Event::Key(k) = ev {
            if k.kind != KeyEventKind::Press {
                continue;
            }
            // Ctrl+C, Esc, and q all cancel cleanly (not an error).
            let cancel = k.code == KeyCode::Esc
                || k.code == KeyCode::Char('q')
                || (k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL));
            if cancel {
                return Ok(SelectorOutcome::Cancelled);
            }
            match k.code {
                KeyCode::Up => move_focus(entries, focus, -1),
                KeyCode::Down => move_focus(entries, focus, 1),
                KeyCode::Char(' ') => {
                    if entries[*focus].detected {
                        entries[*focus].checked = !entries[*focus].checked;
                    }
                }
                KeyCode::Enter => return Ok(SelectorOutcome::Confirmed),
                _ => {}
            }
        }
    }
}

fn move_focus(entries: &[Entry], focus: &mut usize, delta: i32) {
    let n = entries.len() as i32;
    let mut idx = *focus as i32;
    for _ in 0..n {
        idx = (idx + delta + n) % n;
        if entries[idx as usize].detected {
            *focus = idx as usize;
            return;
        }
    }
}

fn render_frame(
    out: &mut std::io::Stdout,
    entries: &[Entry],
    focus: usize,
    _detections: &[Detection],
    version_note: &str,
) -> MineResult<()> {
    let _ = queue!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));
    // Banner (ASCII art) at the top of the alternate screen.
    let _ = queue!(out, Print(super::banner::ASCII_ART));
    let _ = queue!(out, Print("  MINE Is Not Everyone's.\r\n\r\n"));
    let _ = queue!(
        out,
        SetForegroundColor(Color::DarkGrey),
        Print(format!("{version_note}\r\n\r\n")),
        SetForegroundColor(Color::Reset)
    );
    // Selector. The detection state is shown inline per entry (undetected
    // agents are greyed with "(not detected)"), so a separate detection
    // summary would be redundant.
    let _ = queue!(out, Print("Select coding agents to install MINE into:\r\n"));
    for (i, e) in entries.iter().enumerate() {
        let checkbox = if e.checked { "[*]" } else { "[ ]" };
        let focused = i == focus;
        if !e.detected {
            let _ = queue!(
                out,
                SetForegroundColor(Color::DarkGrey),
                Print(format!("{checkbox} {} (not detected)\r\n", e.display)),
                SetForegroundColor(Color::Reset)
            );
        } else if focused {
            let _ = queue!(
                out,
                SetForegroundColor(Color::White),
                Print(format!("{checkbox} {}\r\n", e.display)),
                SetForegroundColor(Color::Reset)
            );
        } else {
            let _ = queue!(
                out,
                SetForegroundColor(Color::Grey),
                Print(format!("{checkbox} {}\r\n", e.display)),
                SetForegroundColor(Color::Reset)
            );
        }
    }
    let _ = queue!(out, Print("\r\n"));
    let _ = queue!(
        out,
        SetForegroundColor(Color::DarkGrey),
        Print("↑/↓ move  ·  Space toggle  ·  Enter confirm  ·  Esc cancel\r\n"),
        SetForegroundColor(Color::Reset)
    );
    Ok(())
}
