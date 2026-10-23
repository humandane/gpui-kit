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

#[cfg(test)]
pub(crate) fn scroll_input(event: ScrollEvent) -> gpui::PlatformInput {
    #[cfg(not(feature = "gpui-fast"))]
    {
        gpui::PlatformInput::Scroll(event)
    }
    #[cfg(feature = "gpui-fast")]
    {
        gpui::PlatformInput::ScrollWheel(event)
    }
}

pub(crate) fn key_binding<A: gpui::Action>(
    keys: &'static str,
    action: A,
    context: Option<&str>,
    cx: &gpui::App,
) -> gpui::KeyBinding {
    #[cfg(not(feature = "gpui-fast"))]
    {
        let predicate = context.map(|context| {
            gpui::KeyBindingContextPredicate::parse(context)
                .expect("built-in keybinding context is valid")
                .into()
        });
        gpui::KeyBinding::load(keys, Box::new(action), predicate, false, None, cx)
            .expect("built-in keybinding is valid for the current host")
    }
    #[cfg(feature = "gpui-fast")]
    {
        let _ = cx;
        gpui::KeyBinding::new(keys, action, context)
    }
}

#[cfg(test)]
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
