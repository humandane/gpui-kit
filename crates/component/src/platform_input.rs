//! Input API compatibility between the pinned upstream fork and GPUI Fast.
#[cfg(not(feature = "gpui-fast"))]
pub(crate) use gpui::ScrollEvent;
#[cfg(feature = "gpui-fast")]
pub(crate) use gpui::ScrollWheelEvent as ScrollEvent;

pub(crate) fn secondary_modifier(modifiers: gpui::Modifiers, cx: &gpui::App) -> bool {
    #[cfg(not(feature = "gpui-fast"))]
    {
        modifiers.secondary(cx)
    }
    #[cfg(feature = "gpui-fast")]
    {
        let _ = cx;
        modifiers.secondary()
    }
}

pub(crate) fn on_scroll<E: gpui::InteractiveElement>(
    element: E,
    listener: impl Fn(&ScrollEvent, &mut gpui::Window, &mut gpui::App) + 'static,
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

pub(crate) fn keystroke_text(stroke: &gpui::Keystroke, cx: &gpui::App) -> String {
    #[cfg(not(feature = "gpui-fast"))]
    {
        stroke.unparse(cx.platform_info().os)
    }
    #[cfg(feature = "gpui-fast")]
    {
        let _ = cx;
        stroke.unparse()
    }
}

pub(crate) fn parse_keystroke(
    key: &str,
    cx: &gpui::App,
) -> Result<gpui::Keystroke, gpui::InvalidKeystrokeError> {
    #[cfg(not(feature = "gpui-fast"))]
    {
        gpui::Keystroke::parse_with_os(key, cx.platform_info().os)
    }
    #[cfg(feature = "gpui-fast")]
    {
        let _ = cx;
        gpui::Keystroke::parse(key)
    }
}
