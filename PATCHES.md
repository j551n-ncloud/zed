# Terax-style patches for Zed

This branch is [Zed](https://github.com/zed-industries/zed) `v1.26.2` with a few small patches that make it behave more like [Terax](https://terax.app), a terminal-first editor.

## What's different

### File tree follows the terminal

When you `cd` in a focused terminal, the project panel switches to that folder. The new folder becomes the only folder in the tree, and the others are removed from the window. It works in both the terminal panel and terminal tabs. It ignores the folder a terminal starts in, skips `/` and your home folder (too big to scan), and does nothing in remote projects.

Code: `follow_terminal_cwd` in `crates/terminal_view/src/terminal_view.rs`.

Each `cd` opens a new folder, so Zed's folder trust check runs again. Trust a parent folder once, for example `~/Documents/Code`, and everything below it is trusted.

### Rendered / Raw switch for Markdown

Markdown files and their previews get a **Rendered | Raw** switch (Raw swaps the preview for the editor in place) at the top right of the toolbar, even when the quick action bar is turned off. Turn on `markdown_preview.open_markdown_files_in_preview` so clicking a `.md` file in the tree opens it rendered.

Code: `crates/zed/src/zed/markdown_mode_toggle.rs`, plus a small public `return_to_editor` method on `MarkdownPreviewView`.

### Clickable path in the status bar

`Home › Documents › … › project` at the bottom left. Clicking a part makes that folder the file tree and `cd`s the active terminal there (only when a shell is waiting at its prompt). Code: `crates/terminal_view/src/cwd_breadcrumb.rs`.

### Background picture with opacity and blur

```jsonc
"background_image": { "path": "~/Pictures/wallpaper.png", "opacity": 0.6, "blur": 0 }
```

Shown behind the whole window; use it with a theme whose backgrounds are partly transparent. `blur` is applied once and cached. Commands: `workspace: select background image`, `workspace: clear background image`. Code: `crates/workspace/src/background_image.rs`.

### Search field and notification bell in the title bar

A search field that opens project search, and a bell that lists terminals which rang their bell while you were looking elsewhere (Claude Code and other agents do this when they finish or need input). Click an entry to jump to that terminal. Code: `crates/terminal_view/src/bell_center.rs`, `crates/title_bar/src/title_bar.rs`.

For Claude Code, add a `Stop` and a `Notification` hook that rings the bell inside Zed:

```json
{ "type": "command", "command": "[ \"$TERM_PROGRAM\" = zed ] && printf '{\"terminalSequence\":\"\\\\"}' || true" }
```

### Builds without full Xcode

`gpui_apple` compiles its Metal shaders at runtime by default (`runtime_shaders`), so the Xcode Command Line Tools are enough to build.

## Build (macOS)

```sh
# Rust and cmake
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
brew install cmake

git clone -b terax https://github.com/j551n-ncloud/zed.git
cd zed
CARGO_PROFILE_RELEASE_FAST_DEBUG=0 cargo build --profile release-fast -p zed
./target/release-fast/zed
```

A first build takes about an hour on an M2 and uses around 40 GB in `target/`.

## Example settings

```jsonc
{
  "terminal": { "starts_open": true },
  "project_panel": { "dock": "left", "starts_open": true },
  "markdown_preview": { "open_markdown_files_in_preview": true }
}
```

## License

Unchanged from upstream Zed. See the `LICENSE-*` files.
