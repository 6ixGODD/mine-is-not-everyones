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

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::style::{Color, SetForegroundColor};
use crossterm::terminal::{self, disable_raw_mode, enable_raw_mode};
use crossterm::{cursor, execute, queue, style::Print, terminal::ClearType};
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
        let final_entries = run_selector(entries)?;
        let mut install = Vec::new();
        let mut uninstall = Vec::new();
        // Build install list from final checked+detected; uninstall list from
        // previously-installed but now unchecked.
        for d in detections {
            let e = final_entries.iter().find(|e| e.slug == d.slug).unwrap();
            if d.detected {
                if e.checked {
                    install.push(d.slug.clone());
                } else if state.record(&d.slug).is_some() {
                    // Was installed, now deselected -> uninstall.
                    uninstall.push(d.slug.clone());
                }
            }
        }
        Ok(SetupPlan { install, uninstall })
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
        })
    }
}

/// Returns true if stdin is a TTY (terminal).
fn is_tty() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal()
}

/// Runs the interactive selector. Returns the final entry states.
fn run_selector(mut entries: Vec<Entry>) -> MineResult<Vec<Entry>> {
    let mut focus: usize = 0;
    // Move focus to the first selectable (detected) entry.
    for (i, e) in entries.iter().enumerate() {
        if e.detected {
            focus = i;
            break;
        }
    }

    enable_raw_mode().map_err(|e| MineError::ExternalDependency {
        detail: format!("TUI: enable_raw_mode failed: {e}"),
    })?;
    let result = selector_loop(&mut entries, &mut focus);
    // Always restore terminal state.
    let _ = disable_raw_mode();
    // Clear the selector lines on exit.
    let mut out = stdout();
    let _ = queue!(out, terminal::Clear(ClearType::FromCursorDown),);
    let _ = out.flush();
    result?;
    Ok(entries)
}

fn selector_loop(entries: &mut [Entry], focus: &mut usize) -> MineResult<()> {
    let mut out = stdout();
    loop {
        // Render.
        let _ = execute!(
            out,
            cursor::MoveTo(0, cursor::position().ok().map(|p| p.1).unwrap_or(0))
        );
        // Simpler: just print lines and reposition each frame.
        render_frame(&mut out, entries, *focus)?;
        out.flush().ok();

        // Read a key.
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
            match k.code {
                KeyCode::Up => move_focus(entries, focus, -1),
                KeyCode::Down => move_focus(entries, focus, 1),
                KeyCode::Char(' ') => {
                    if entries[*focus].detected {
                        entries[*focus].checked = !entries[*focus].checked;
                    }
                }
                KeyCode::Enter => return Ok(()),
                KeyCode::Char('q') | KeyCode::Esc => {
                    return Err(MineError::ExternalDependency {
                        detail: "setup cancelled by user".to_string(),
                    });
                }
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

fn render_frame(out: &mut std::io::Stdout, entries: &[Entry], focus: usize) -> MineResult<()> {
    // Move cursor up to redraw the same block. We render `entries.len()+2`
    // lines (header + entries + footer). Track how many we printed last frame
    // is hard in raw mode without absolute positioning; simpler: move up by
    // the frame height each redraw.
    let height = entries.len() as u16 + 3;
    let _ = queue!(
        out,
        cursor::MoveUp(height.saturating_sub(1)),
        terminal::Clear(ClearType::FromCursorDown),
    );
    let _ = queue!(out, Print("Select coding agents to install MINE into:\r\n"));
    for (i, e) in entries.iter().enumerate() {
        let checkbox = if e.checked { "[*]" } else { "[ ]" };
        let line = format!("{checkbox} {}\r\n", e.display);
        let focused = i == focus;
        if !e.detected {
            // Greyed out, undetected.
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
                Print(line),
                SetForegroundColor(Color::Reset)
            );
        } else {
            let _ = queue!(
                out,
                SetForegroundColor(Color::Grey),
                Print(line),
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
