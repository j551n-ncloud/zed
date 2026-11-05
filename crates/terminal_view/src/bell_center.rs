//! Local patch: Terax-style notifications. Collects bells from terminals you
//! are not looking at (Claude Code and other agents ring the terminal bell
//! when they finish or need input) so the title bar can list them and jump
//! back to the right terminal.

use gpui::{App, AppContext as _, Context, Entity, Focusable as _, Global, SharedString, WeakEntity, Window};
use workspace::{Pane, Workspace};

use crate::{TerminalView, terminal_panel::TerminalPanel};

pub struct BellEntry {
    pub terminal: WeakEntity<TerminalView>,
    pub title: SharedString,
    /// What happened, for example "needs your input" or "finished".
    pub message: SharedString,
}

#[derive(Default)]
pub struct BellCenter {
    entries: Vec<BellEntry>,
}

struct GlobalBellCenter(Entity<BellCenter>);

impl Global for GlobalBellCenter {}

impl BellCenter {
    pub fn global(cx: &mut App) -> Entity<BellCenter> {
        if let Some(global) = cx.try_global::<GlobalBellCenter>() {
            return global.0.clone();
        }
        let center = cx.new(|_| BellCenter::default());
        cx.set_global(GlobalBellCenter(center.clone()));
        center
    }

    /// Newest first. Terminals that were closed are skipped.
    pub fn entries(&self) -> impl Iterator<Item = &BellEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.terminal.upgrade().is_some())
    }

    pub fn count(&self) -> usize {
        self.entries().count()
    }

    pub fn clear_all(&mut self, cx: &mut Context<Self>) {
        self.entries.clear();
        cx.notify();
    }

    /// A plain terminal bell. Does not replace a more specific notification
    /// that is already listed for the same terminal.
    pub(crate) fn ring(terminal: WeakEntity<TerminalView>, title: SharedString, cx: &mut App) {
        let center = Self::global(cx);
        center.update(cx, |center, cx| {
            center.entries.retain(|entry| entry.terminal.upgrade().is_some());
            if center.entries.iter().any(|entry| entry.terminal == terminal) {
                return;
            }
            center.entries.insert(
                0,
                BellEntry {
                    terminal,
                    title,
                    message: "rang the bell".into(),
                },
            );
            cx.notify();
        });
    }

    /// A desktop notification (OSC 777 / OSC 9). Terax's Claude Code hooks
    /// send `working`, `attention` and `finished`.
    pub(crate) fn notify(
        terminal: WeakEntity<TerminalView>,
        terminal_title: SharedString,
        title: Option<String>,
        body: String,
        looking: bool,
        window_active: bool,
        cx: &mut App,
    ) {
        let (message, sound): (SharedString, &str) = match body.as_str() {
            "working" => {
                Self::clear(&terminal, cx);
                return;
            }
            "attention" => ("needs your input".into(), "Ping"),
            "finished" => ("finished".into(), "Glass"),
            "" => (title.clone().unwrap_or_default().into(), "Glass"),
            other => (other.to_owned().into(), "Glass"),
        };
        if looking {
            return;
        }

        let center = Self::global(cx);
        center.update(cx, |center, cx| {
            center
                .entries
                .retain(|entry| entry.terminal != terminal && entry.terminal.upgrade().is_some());
            center.entries.insert(
                0,
                BellEntry {
                    terminal,
                    title: terminal_title.clone(),
                    message: message.clone(),
                },
            );
            cx.notify();
        });

        play_sound(sound);
        if !window_active {
            show_banner(&terminal_title, &message);
        }
    }

    pub(crate) fn clear(terminal: &WeakEntity<TerminalView>, cx: &mut App) {
        let Some(center) = cx.try_global::<GlobalBellCenter>().map(|global| global.0.clone())
        else {
            return;
        };
        center.update(cx, |center, cx| {
            let before = center.entries.len();
            center.entries.retain(|entry| &entry.terminal != terminal);
            if center.entries.len() != before {
                cx.notify();
            }
        });
    }
}

/// Brings a terminal into view and focuses it, whether it is a tab in the
/// center or lives in the terminal panel.
pub fn reveal_terminal(
    terminal: Entity<TerminalView>,
    workspace: &mut Workspace,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    if !workspace.activate_item(&terminal, true, true, window, cx) {
        let Some(panel) = workspace.panel::<TerminalPanel>(cx) else {
            return;
        };
        workspace.open_panel::<TerminalPanel>(window, cx);
        let panes: Vec<Entity<Pane>> = panel.read(cx).panes().into_iter().cloned().collect();
        for pane in panes {
            if let Some(index) = pane.read(cx).index_for_item(&terminal) {
                pane.update(cx, |pane, cx| pane.activate_item(index, true, true, window, cx));
                break;
            }
        }
    }
    window.focus(&terminal.focus_handle(cx), cx);
    BellCenter::clear(&terminal.downgrade(), cx);
}

/// Plays one of the macOS system sounds in /System/Library/Sounds.
fn play_sound(name: &str) {
    #[cfg(target_os = "macos")]
    {
        let path = format!("/System/Library/Sounds/{name}.aiff");
        std::process::Command::new("afplay").arg(path).spawn().ok();
    }
    #[cfg(not(target_os = "macos"))]
    let _ = name;
}

/// Shows a macOS notification banner.
fn show_banner(title: &str, message: &str) {
    #[cfg(target_os = "macos")]
    {
        let quote = |text: &str| text.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!(
            "display notification \"{}\" with title \"Zed\" subtitle \"{}\"",
            quote(message),
            quote(title)
        );
        std::process::Command::new("osascript")
            .arg("-e")
            .arg(script)
            .spawn()
            .ok();
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (title, message);
}
