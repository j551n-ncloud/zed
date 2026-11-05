//! Local patch: a Terax-style clickable path in the status bar,
//! `Home › Documents › … › project`. Clicking a part makes that folder the
//! project panel root and `cd`s the active terminal there when a shell is
//! waiting at its prompt.

use std::path::{Path, PathBuf};

use gpui::{Entity, Subscription, WeakEntity};
use project::Project;
use ui::prelude::*;
use workspace::{HideStatusItem, StatusItemView, Workspace, item::ItemHandle};

use crate::{TerminalView, set_project_root, terminal_panel::TerminalPanel};

const SHELLS: &[&str] = &["zsh", "bash", "fish", "sh", "dash", "nu", "elvish", "xonsh"];

pub struct CwdBreadcrumb {
    workspace: WeakEntity<Workspace>,
    project: Entity<Project>,
    _project_subscription: Subscription,
}

impl CwdBreadcrumb {
    pub fn new(workspace: &Workspace, cx: &mut Context<Self>) -> Self {
        let project = workspace.project().clone();
        let _project_subscription = cx.subscribe(&project, |_, _, _: &project::Event, cx| {
            cx.notify();
        });
        Self {
            workspace: workspace.weak_handle(),
            project,
            _project_subscription,
        }
    }
}

/// `(label, path)` pairs from the top (Home or `/`) down to `root`.
fn crumbs(root: &Path) -> Vec<(SharedString, PathBuf)> {
    let home = util::paths::home_dir();
    let mut crumbs = Vec::new();
    let mut current = Some(root);
    while let Some(path) = current {
        if path == home.as_path() {
            crumbs.push(("Home".into(), path.to_path_buf()));
            break;
        }
        let label: SharedString = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string().into(),
            None => "/".into(),
        };
        crumbs.push((label, path.to_path_buf()));
        current = path.parent();
    }
    crumbs.reverse();
    crumbs
}

fn navigate(
    workspace: &WeakEntity<Workspace>,
    project: &Entity<Project>,
    path: PathBuf,
    cx: &mut App,
) {
    set_project_root(project.clone(), path.clone(), cx);

    let Some(workspace) = workspace.upgrade() else {
        return;
    };
    // Prefer a terminal tab in the center, then the terminal panel.
    let terminal_view = workspace
        .read(cx)
        .active_item(cx)
        .and_then(|item| item.downcast::<TerminalView>())
        .or_else(|| {
            let panel = workspace.read(cx).panel::<TerminalPanel>(cx)?;
            let pane = panel.read(cx).active_pane.clone();
            let item = pane.read(cx).active_item()?;
            item.downcast::<TerminalView>()
        });
    let Some(terminal_view) = terminal_view else {
        return;
    };
    let terminal = terminal_view.read(cx).terminal().clone();
    let at_prompt = terminal
        .read(cx)
        .foreground_process_command_name()
        .is_some_and(|name| SHELLS.contains(&name.trim_start_matches('-')));
    if !at_prompt {
        return;
    }
    let quoted = path.to_string_lossy().replace('\'', "'\\''");
    terminal.update(cx, |terminal, _| {
        terminal.input(format!("cd -- '{quoted}'\r").into_bytes());
    });
}

impl Render for CwdBreadcrumb {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = self
            .project
            .read(cx)
            .visible_worktrees(cx)
            .next()
            .map(|worktree| worktree.read(cx).abs_path().to_path_buf());
        let Some(root) = root else {
            return div().into_any_element();
        };

        let crumbs = crumbs(&root);
        let last = crumbs.len().saturating_sub(1);
        let mut row = h_flex().gap_0p5();
        for (ix, (label, path)) in crumbs.into_iter().enumerate() {
            if ix > 0 {
                row = row.child(
                    Icon::new(IconName::ChevronRight)
                        .size(IconSize::XSmall)
                        .color(Color::Muted),
                );
            }
            let is_home = label.as_ref() == "Home" && ix == 0;
            let workspace = self.workspace.clone();
            let project = self.project.clone();
            let button = Button::new(("cwd-crumb", ix), label)
                .label_size(LabelSize::Small)
                .style(ButtonStyle::Subtle)
                .color(if ix == last {
                    Color::Default
                } else {
                    Color::Muted
                })
                .when(is_home, |button| {
                    button.start_icon(Icon::new(IconName::Folder).size(IconSize::XSmall))
                })
                .on_click(move |_, _window, cx| navigate(&workspace, &project, path.clone(), cx));
            row = row.child(button);
        }
        row.into_any_element()
    }
}

impl StatusItemView for CwdBreadcrumb {
    fn set_active_pane_item(
        &mut self,
        _active_pane_item: Option<&dyn ItemHandle>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.notify();
    }

    fn hide_setting(&self, _: &App) -> Option<HideStatusItem> {
        None
    }
}
