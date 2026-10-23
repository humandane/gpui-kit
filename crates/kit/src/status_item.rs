//! A retained native menu bar item, independent of any application window.
//!
//! macOS is currently supported. Creation on another platform or a deterministic
//! test application returns [`StatusItemError::Unsupported`]. Retain the handle
//! for as long as the item should exist; dropping it removes the item and cancels
//! queued activation callbacks. No application or recording policy lives here.

use std::{fmt, sync::Arc};

use crate::{App, Bounds, DisplayId, Pixels, SharedString, Size, Task, point, px, size};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(target_os = "macos"))]
mod unsupported;
#[cfg(target_os = "macos")]
use macos as native;
#[cfg(not(target_os = "macos"))]
use unsupported as native;

/// Encoded image bytes and their intended logical size. Template images let the
/// operating system choose a legible foreground for the menu bar appearance.
#[derive(Clone, Debug)]
pub struct StatusItemImage {
    bytes: Arc<[u8]>,
    size: Size<Pixels>,
    template: bool,
}

impl StatusItemImage {
    /// Accepts an OS-decodable image, such as PNG or PDF. Defaults to a template.
    /// Decode and positive, finite size validation happen when the image is set.
    pub fn new(bytes: impl Into<Arc<[u8]>>, size: Size<Pixels>) -> Self {
        Self {
            bytes: bytes.into(),
            size,
            template: true,
        }
    }

    pub fn with_template(mut self, template: bool) -> Self {
        self.template = template;
        self
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn size(&self) -> Size<Pixels> {
        self.size
    }
    pub fn is_template(&self) -> bool {
        self.template
    }

    fn validate(&self) -> Result<(), StatusItemError> {
        let width = f32::from(self.size.width);
        let height = f32::from(self.size.height);
        if self.bytes.is_empty()
            || !width.is_finite()
            || !height.is_finite()
            || width <= 0.
            || height <= 0.
        {
            return Err(StatusItemError::InvalidImage);
        }
        Ok(())
    }
}

/// Initial presentation of a square, icon-only native item.
#[derive(Clone, Debug)]
pub struct StatusItemOptions {
    image: StatusItemImage,
    accessible_name: SharedString,
    tooltip: Option<SharedString>,
}

impl StatusItemOptions {
    pub fn new(image: StatusItemImage, accessible_name: impl Into<SharedString>) -> Self {
        Self {
            image,
            accessible_name: accessible_name.into(),
            tooltip: None,
        }
    }

    pub fn with_tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn image(&self) -> &StatusItemImage {
        &self.image
    }
    pub fn accessible_name(&self) -> &str {
        &self.accessible_name
    }
    pub fn tooltip(&self) -> Option<&str> {
        self.tooltip.as_deref()
    }
}

/// Current native geometry. All bounds are logical pixels relative to the
/// top-left of this display, matching macOS GPUI `WindowOptions` coordinates.
/// They are not AppKit's global bottom-left coordinates or physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatusItemAnchor {
    display_id: DisplayId,
    bounds: Bounds<Pixels>,
    usable_bounds: Bounds<Pixels>,
    scale_factor: f32,
}

impl StatusItemAnchor {
    pub fn display_id(&self) -> DisplayId {
        self.display_id
    }
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }
    pub fn usable_bounds(&self) -> Bounds<Pixels> {
        self.usable_bounds
    }
    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    /// Centers a popup below the item and clamps it to the usable display.
    /// Oversized content is constrained to the display; the view must handle
    /// that available size. Invalid requested dimensions become zero.
    pub fn popup_bounds(&self, requested: Size<Pixels>) -> Bounds<Pixels> {
        let finite = |value: Pixels| {
            let value = f32::from(value);
            px(if value.is_finite() { value.max(0.) } else { 0. })
        };
        let width = finite(requested.width).min(self.usable_bounds.size.width);
        let height = finite(requested.height).min(self.usable_bounds.size.height);
        let origin = point(
            (self.bounds.origin.x + (self.bounds.size.width - width) / 2.)
                .max(self.usable_bounds.origin.x)
                .min(self.usable_bounds.right() - width),
            self.bounds
                .bottom()
                .max(self.usable_bounds.origin.y)
                .min(self.usable_bounds.bottom() - height),
        );
        Bounds::new(origin, size(width, height))
    }
}

/// Failure to create or update the native item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StatusItemError {
    Unsupported,
    NotMainThread,
    InvalidImage,
    MissingButton,
}

impl fmt::Display for StatusItemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unsupported => "native status items are unavailable on this platform",
            Self::NotMainThread => "native status items must be created on the main thread",
            Self::InvalidImage => "the status image could not be decoded or has an invalid size",
            Self::MissingButton => "the system did not supply a status item button",
        })
    }
}
impl std::error::Error for StatusItemError {}

/// Owns a native item and its activation delivery. This handle cannot be sent
/// between threads. All native work stays on the creating foreground thread.
pub struct StatusItem {
    native: native::StatusItem,
    _activations: Task<()>,
}

impl StatusItem {
    /// Activation is delivered once on a subsequent GPUI foreground turn, never
    /// synchronously from inside AppKit. Capture weak entities in the callback
    /// when an entity owns this handle, to avoid an ownership cycle.
    pub fn new(
        options: StatusItemOptions,
        cx: &mut App,
        on_activate: impl FnMut(&mut App) + 'static,
    ) -> Result<Self, StatusItemError> {
        if cx.is_test() {
            return Err(StatusItemError::Unsupported);
        }
        options.image.validate()?;
        let (sender, receiver) = async_channel::unbounded();
        let native = native::StatusItem::new(options, sender)?;
        Ok(Self {
            native,
            _activations: activation_task(receiver, cx, on_activate),
        })
    }

    pub fn set_image(&mut self, image: StatusItemImage) -> Result<(), StatusItemError> {
        image.validate()?;
        self.native.set_image(image)
    }
    pub fn set_tooltip(&mut self, tooltip: Option<&str>) {
        self.native.set_tooltip(tooltip);
    }
    pub fn set_accessible_name(&mut self, name: &str) {
        self.native.set_accessible_name(name);
    }
    pub fn set_selected(&mut self, selected: bool) {
        self.native.set_selected(selected);
    }

    /// Returns no anchor when the native item has no visible window/display.
    /// Read again before opening and while an attached popup is visible; dismiss
    /// or reposition when the snapshot changes. Do not cache it across displays.
    pub fn anchor(&self) -> Option<StatusItemAnchor> {
        self.native.anchor()
    }
}

fn activation_task(
    receiver: async_channel::Receiver<()>,
    cx: &App,
    mut on_activate: impl FnMut(&mut App) + 'static,
) -> Task<()> {
    cx.spawn(async move |cx| {
        while receiver.recv().await.is_ok() {
            cx.update(&mut on_activate);
        }
    })
}

// AppKit global bottom-left rectangle -> display-local top-left rectangle.
// Kept independent of AppKit types so multi-display origins can be tested.
#[cfg(any(target_os = "macos", test))]
fn display_local_bounds(rect: Bounds<Pixels>, screen: Bounds<Pixels>) -> Bounds<Pixels> {
    Bounds::new(
        point(
            rect.origin.x - screen.origin.x,
            screen.bottom() - rect.bottom(),
        ),
        rect.size,
    )
}

#[cfg(test)]
mod tests {
    use super::{StatusItemAnchor, StatusItemImage, display_local_bounds};
    use crate::{Bounds, DisplayId, point, px, size};

    fn bounds(x: f32, y: f32, width: f32, height: f32) -> Bounds<crate::Pixels> {
        Bounds::new(point(px(x), px(y)), size(px(width), px(height)))
    }

    #[test]
    fn converts_secondary_display_origins_without_applying_scale_twice() {
        let screen = bounds(-1920., 300., 1920., 1080.);
        assert_eq!(
            display_local_bounds(bounds(-100., 1356., 22., 24.), screen),
            bounds(1820., 0., 22., 24.)
        );
        assert_eq!(
            display_local_bounds(bounds(-1920., 300., 1920., 1056.), screen),
            bounds(0., 24., 1920., 1056.)
        );
    }

    #[test]
    fn popup_respects_both_edges_and_oversized_content() {
        let mut anchor = StatusItemAnchor {
            display_id: DisplayId::new(1),
            bounds: bounds(980., 0., 20., 24.),
            usable_bounds: bounds(0., 24., 1000., 700.),
            scale_factor: 2.,
        };
        assert_eq!(
            anchor.popup_bounds(size(px(320.), px(300.))),
            bounds(680., 24., 320., 300.)
        );
        anchor.bounds.origin.x = px(0.);
        assert_eq!(
            anchor.popup_bounds(size(px(320.), px(300.))),
            bounds(0., 24., 320., 300.)
        );
        assert_eq!(
            anchor.popup_bounds(size(px(1400.), px(900.))),
            anchor.usable_bounds
        );
    }

    #[test]
    fn image_rejects_non_finite_or_empty_dimensions_before_native_work() {
        assert!(
            StatusItemImage::new(&b"image"[..], size(px(f32::NAN), px(18.)))
                .validate()
                .is_err()
        );
        assert!(
            StatusItemImage::new(&b"image"[..], size(px(18.), px(0.)))
                .validate()
                .is_err()
        );
        assert!(
            StatusItemImage::new(&b""[..], size(px(18.), px(18.)))
                .validate()
                .is_err()
        );
    }

    #[crate::test]
    fn activation_is_deferred_and_cancelled_with_its_owner(cx: &mut crate::TestAppContext) {
        use std::{cell::Cell, rc::Rc};
        let count = Rc::new(Cell::new(0));
        let observed = count.clone();
        let (sender, receiver) = async_channel::unbounded();
        let task = cx.update(|cx| {
            super::activation_task(receiver, cx, move |_| {
                observed.set(observed.get() + 1);
            })
        });
        sender.try_send(()).expect("queue first activation");
        sender.try_send(()).expect("queue second activation");
        assert_eq!(
            count.get(),
            0,
            "native callback must not synchronously enter GPUI"
        );
        cx.run_until_parked();
        assert_eq!(count.get(), 2, "each activation is delivered exactly once");
        sender
            .try_send(())
            .expect("queue activation before owner drops");
        drop(task);
        cx.run_until_parked();
        assert_eq!(count.get(), 2, "dropping the owner cancels queued work");
        assert!(sender.try_send(()).is_err());
    }
}
