//! Local patch: a "Rendered | Raw" switch in the toolbar of Markdown files
//! and their previews. Shown even when the quick action bar is turned off.

use editor::Editor;
use gpui::{Entity, EventEmitter, WeakEntity};
use markdown_preview::markdown_preview_view::MarkdownPreviewView;
use ui::prelude::*;
use workspace::{
    ToolbarItemEvent, ToolbarItemLocation, ToolbarItemView, Workspace, item::ItemHandle,
};

enum Mode {
    Raw(Entity<Editor>),
    Rendered(Entity<MarkdownPreviewView>),
}

pub struct MarkdownModeToggle {
    workspace: WeakEntity<Workspace>,
    mode: Option<Mode>,
}

impl MarkdownModeToggle {
    pub fn new(workspace: &Workspace) -> Self {
        Self {
            workspace: workspace.weak_handle(),
            mode: None,
        }
    }
}

impl Render for MarkdownModeToggle {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let Some(mode) = self.mode.as_ref() else {
            return div().into_any_element();
        };
        let (editor, preview) = match mode {
            Mode::Raw(editor) => (Some(editor.clone()), None),
            Mode::Rendered(preview) => (None, Some(preview.clone())),
        };
        let rendered = preview.is_some();
        let workspace = self.workspace.clone();

        h_flex()
            .gap_0p5()
            .child(
                Button::new("markdown-mode-rendered", "Rendered")
                    .label_size(LabelSize::Small)
                    .style(ButtonStyle::Subtle)
                    .toggle_state(rendered)
                    .on_click(move |_, window, cx| {
                        let (Some(editor), Some(workspace)) = (editor.clone(), workspace.upgrade())
                        else {
                            return;
                        };
                        workspace.update(cx, |workspace, cx| {
                            let Some(pane) = workspace.pane_for(&editor) else {
                                return;
                            };
                            MarkdownPreviewView::open_preview_in_pane(
                                workspace, editor, pane, window, cx,
                            );
                        });
                    }),
            )
            .child(
                Button::new("markdown-mode-raw", "Raw")
                    .label_size(LabelSize::Small)
                    .style(ButtonStyle::Subtle)
                    .toggle_state(!rendered)
                    .on_click(move |_, window, cx| {
                        let Some(preview) = preview.clone() else {
                            return;
                        };
                        preview.update(cx, |preview, cx| preview.return_to_editor(window, cx));
                    }),
            )
            .into_any_element()
    }
}

impl EventEmitter<ToolbarItemEvent> for MarkdownModeToggle {}

impl ToolbarItemView for MarkdownModeToggle {
    fn set_active_pane_item(
        &mut self,
        active_pane_item: Option<&dyn ItemHandle>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> ToolbarItemLocation {
        self.mode = active_pane_item.and_then(|item| {
            if let Some(preview) = item.downcast::<MarkdownPreviewView>() {
                return Some(Mode::Rendered(preview));
            }
            let editor = item.downcast::<Editor>()?;
            MarkdownPreviewView::is_markdown_file(&editor, cx).then_some(Mode::Raw(editor))
        });
        cx.notify();
        if self.mode.is_some() {
            ToolbarItemLocation::PrimaryRight
        } else {
            ToolbarItemLocation::Hidden
        }
    }
}
