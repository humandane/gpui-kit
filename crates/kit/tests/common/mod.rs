//! Fixtures use the production window entry point with deterministic test bounds.
#![allow(dead_code)]
use gpui_kit::{
    AnyWindowHandle, App, AppContext, Bounds, Context, Entity, Pixels, Point, Render, Size,
    TestAppContext, Window, WindowBounds, WindowHandle, WindowOptions, base::Root,
};

pub fn open_window<V: Render>(
    cx: &mut TestAppContext,
    size: Option<Size<Pixels>>,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> (WindowHandle<Root>, Entity<V>) {
    cx.update(|cx| {
        let bounds = size
            .map(|size| Bounds {
                origin: Point::default(),
                size,
            })
            .unwrap_or_else(|| Bounds::maximized(None, cx));
        let (window, content) = gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            cx,
            build,
        )
        .expect("open test window");
        (window.downcast::<Root>().expect("Base Root"), content)
    })
}

pub fn update_content<V: Render, R>(
    window: impl Into<AnyWindowHandle>,
    content: &Entity<V>,
    cx: &mut TestAppContext,
    update: impl FnOnce(&mut V, &mut Window, &mut Context<V>) -> R,
) -> gpui_kit::Result<R> {
    cx.update_window(window.into(), |_, window, cx| {
        content.update(cx, |view, cx| update(view, window, cx))
    })
}

// Keep test events on the same input API as the selected GPUI backend.
#[cfg(not(feature = "gpui-fast"))]
pub use gpui_kit::ScrollEvent;
#[cfg(feature = "gpui-fast")]
pub use gpui_kit::ScrollWheelEvent as ScrollEvent;

pub fn parse_keystroke(key: &str, cx: &App) -> gpui_kit::Keystroke {
    #[cfg(not(feature = "gpui-fast"))]
    let result = gpui_kit::Keystroke::parse_with_os(key, cx.platform_info().os);
    #[cfg(feature = "gpui-fast")]
    let result = {
        let _ = cx;
        gpui_kit::Keystroke::parse(key)
    };
    result.expect("valid test keystroke")
}

pub fn secondary_key(cx: &App) -> gpui_kit::Modifiers {
    #[cfg(not(feature = "gpui-fast"))]
    {
        gpui_kit::Modifiers::secondary_key(cx)
    }
    #[cfg(feature = "gpui-fast")]
    {
        let _ = cx;
        gpui_kit::Modifiers::secondary_key()
    }
}

pub fn on_scroll<E: gpui_kit::InteractiveElement>(
    element: E,
    listener: impl Fn(&ScrollEvent, &mut Window, &mut App) + 'static,
) -> E {
    #[cfg(not(feature = "gpui-fast"))]
    {
        element.on_scroll(listener)
    }
    #[cfg(feature = "gpui-fast")]
    {
        element.on_scroll_wheel(listener)
    }
}
