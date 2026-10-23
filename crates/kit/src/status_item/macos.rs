use async_channel::Sender;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{
    AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel,
};
use objc2_app_kit::{
    NSAccessibility, NSImage, NSSquareStatusItemLength, NSStatusBar, NSStatusBarButton,
    NSStatusItem,
};
use objc2_foundation::{NSData, NSNumber, NSRect, NSSize, NSString, ns_string};

use super::{
    StatusItemAnchor, StatusItemError, StatusItemImage, StatusItemOptions, display_local_bounds,
};
use crate::{Bounds, DisplayId, Pixels, point, px, size};

struct TargetIvars {
    activations: Sender<()>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "GPUIKitStatusItemTarget"]
    #[thread_kind = MainThreadOnly]
    #[ivars = TargetIvars]
    struct Target;

    impl Target {
        #[unsafe(method(activate:))]
        fn activate(&self, _sender: &AnyObject) {
            // The queue never blocks the native callback or borrows GPUI. A
            // disconnected receiver means the owner is already being dropped.
            if self.ivars().activations.try_send(()).is_err() {
                return;
            }
        }
    }
);

impl Target {
    fn new(activations: Sender<()>, mtm: MainThreadMarker) -> Retained<Self> {
        let allocated = Self::alloc(mtm).set_ivars(TargetIvars { activations });
        // SAFETY: NSObject initialization of our freshly allocated subclass.
        unsafe { msg_send![super(allocated), init] }
    }
}

pub(super) struct StatusItem {
    bar: Retained<NSStatusBar>,
    item: Retained<NSStatusItem>,
    button: Retained<NSStatusBarButton>,
    _target: Retained<Target>,
}

impl StatusItem {
    pub(super) fn new(
        options: StatusItemOptions,
        activations: Sender<()>,
    ) -> Result<Self, StatusItemError> {
        let mtm = MainThreadMarker::new().ok_or(StatusItemError::NotMainThread)?;
        let image = decode_image(&options.image)?;
        let bar = NSStatusBar::systemStatusBar();
        let item = bar.statusItemWithLength(NSSquareStatusItemLength);
        let Some(button) = item.button(mtm) else {
            bar.removeStatusItem(&item);
            return Err(StatusItemError::MissingButton);
        };
        let target = Target::new(activations, mtm);
        button.setImage(Some(&image));
        button.setAccessibilityLabel(Some(&NSString::from_str(&options.accessible_name)));
        button.setToolTip(
            options
                .tooltip
                .as_ref()
                .map(|text| NSString::from_str(text))
                .as_deref(),
        );
        // SAFETY: target is retained for the entire item's lifetime and the
        // selector has the AppKit target/action signature on that exact class.
        unsafe {
            button.setTarget(Some(&target));
            button.setAction(Some(sel!(activate:)));
        }
        Ok(Self {
            bar,
            item,
            button,
            _target: target,
        })
    }

    pub(super) fn set_image(&mut self, image: StatusItemImage) -> Result<(), StatusItemError> {
        // Decode before replacing so a failed update preserves the old image.
        let image = decode_image(&image)?;
        self.button.setImage(Some(&image));
        Ok(())
    }
    pub(super) fn set_tooltip(&mut self, text: Option<&str>) {
        self.button
            .setToolTip(text.map(NSString::from_str).as_deref());
    }
    pub(super) fn set_accessible_name(&mut self, text: &str) {
        self.button
            .setAccessibilityLabel(Some(&NSString::from_str(text)));
    }
    pub(super) fn set_selected(&mut self, selected: bool) {
        self.button.highlight(selected);
    }
    pub(super) fn anchor(&self) -> Option<StatusItemAnchor> {
        if !self.item.isVisible() {
            return None;
        }
        let window = self.button.window()?;
        if !window.isVisible() {
            return None;
        }
        let screen = window.screen()?;
        let description = screen.deviceDescription();
        let number = description.objectForKey(ns_string!("NSScreenNumber"))?;
        let number = number.downcast_ref::<NSNumber>()?;
        let screen_bounds = bounds(screen.frame());
        let rect = self.button.convertRect_toView(self.button.bounds(), None);
        Some(StatusItemAnchor {
            display_id: DisplayId::new(u64::from(number.unsignedIntValue())),
            bounds: display_local_bounds(bounds(window.convertRectToScreen(rect)), screen_bounds),
            usable_bounds: display_local_bounds(bounds(screen.visibleFrame()), screen_bounds),
            scale_factor: screen.backingScaleFactor() as f32,
        })
    }
}

impl Drop for StatusItem {
    fn drop(&mut self) {
        // SAFETY: clear native references before releasing the retained target.
        unsafe {
            self.button.setAction(None);
            self.button.setTarget(None);
        }
        self.bar.removeStatusItem(&self.item);
    }
}

fn decode_image(image: &StatusItemImage) -> Result<Retained<NSImage>, StatusItemError> {
    let data = NSData::with_bytes(&image.bytes);
    let native =
        NSImage::initWithData(NSImage::alloc(), &data).ok_or(StatusItemError::InvalidImage)?;
    native.setSize(NSSize::new(
        f32::from(image.size.width) as f64,
        f32::from(image.size.height) as f64,
    ));
    native.setTemplate(image.template);
    Ok(native)
}

fn bounds(rect: NSRect) -> Bounds<Pixels> {
    Bounds::new(
        point(px(rect.origin.x as f32), px(rect.origin.y as f32)),
        size(px(rect.size.width as f32), px(rect.size.height as f32)),
    )
}
