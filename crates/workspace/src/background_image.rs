//! Local patch: a Terax-style picture behind the whole window, with opacity
//! and blur. Configured with `background_image` in settings, or with the
//! `workspace: select background image` command.

use std::{
    collections::HashSet,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    sync::Mutex,
};

use gpui::{AnyElement, Context, ObjectFit, Styled as _, div, img, prelude::*};
use settings::Settings as _;

use crate::{Workspace, WorkspaceSettings};

/// Blur jobs already started, so each picture is blurred only once.
static STARTED: Mutex<Option<HashSet<u64>>> = Mutex::new(None);

pub(crate) fn expand_path(path: &str) -> PathBuf {
    let home = util::paths::home_dir();
    if path == "~" {
        home.clone()
    } else if let Some(rest) = path.strip_prefix("~/") {
        home.join(rest)
    } else {
        PathBuf::from(path)
    }
}

/// The picture layer, or `None` when no picture is set (or a blurred copy
/// is still being prepared).
pub(crate) fn render_background(cx: &mut Context<Workspace>) -> Option<AnyElement> {
    let settings = WorkspaceSettings::get_global(cx).background_image.clone()?;
    let source = expand_path(settings.path.as_deref()?.trim());
    let opacity = settings.opacity.unwrap_or(0.5).clamp(0.0, 1.0);
    let blur = settings.blur.unwrap_or(0.0).max(0.0);
    if opacity <= 0.0 {
        return None;
    }
    let shown = if blur < 0.5 {
        source
    } else {
        blurred_copy(source, blur, cx)?
    };
    Some(
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .opacity(opacity)
            .child(img(shown).size_full().object_fit(ObjectFit::Cover))
            .into_any_element(),
    )
}

fn blurred_copy(source: PathBuf, blur: f32, cx: &mut Context<Workspace>) -> Option<PathBuf> {
    let modified = std::fs::metadata(&source).and_then(|m| m.modified()).ok();
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    blur.to_bits().hash(&mut hasher);
    modified.hash(&mut hasher);
    let key = hasher.finish();
    let target = std::env::temp_dir().join(format!("zed-background-{key:016x}.png"));
    if target.exists() {
        return Some(target);
    }

    {
        let mut started = STARTED.lock().ok()?;
        if !started.get_or_insert_with(HashSet::new).insert(key) {
            return None;
        }
    }
    let job = cx.background_spawn(async move { blur_to_file(&source, blur, &target) });
    cx.spawn(async move |workspace, cx| {
        if let Err(error) = job.await {
            log::error!("background image: {error:#}");
        }
        workspace.update(cx, |_, cx| cx.notify()).ok();
    })
    .detach();
    None
}

fn blur_to_file(source: &Path, blur: f32, target: &Path) -> anyhow::Result<()> {
    let picture = image::open(source)?;
    // A blurred background does not need more than ~1920px.
    let picture = if picture.width() > 1920 {
        picture.resize(1920, u32::MAX, image::imageops::FilterType::Triangle)
    } else {
        picture
    };
    let blurred = image::imageops::fast_blur(&picture.to_rgba8(), blur);
    let partial = target.with_extension("partial.png");
    blurred.save(&partial)?;
    std::fs::rename(&partial, target)?;
    Ok(())
}
