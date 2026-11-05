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

    pub(crate) fn ring(terminal: WeakEntity<TerminalView>, title: SharedString, cx: &mut App) {
        let center = Self::global(cx);
        center.update(cx, |center, cx| {
            center
                .entries
                .retain(|entry| entry.terminal != terminal && entry.terminal.upgrade().is_some());
            center.entries.insert(0, BellEntry { terminal, title });
            cx.notify();
        });
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
