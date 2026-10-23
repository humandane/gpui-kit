use std::time::{Duration, Instant};

use gpui_kit::component::{
    ActiveTheme, Disableable, StyledExt, button::Button, h_flex, scroll::ScrollableElement, v_flex,
};
use gpui_kit::login_item::{LoginItem, current_launch_reason};
use gpui_kit::status_item::{StatusItem, StatusItemAnchor, StatusItemImage, StatusItemOptions};
use gpui_kit::*;

gpui_kit::actions!(status_item_example, [Quit, DismissPopup]);

/// The main window retains the session. Both views share this one counter and
/// demo state; a native callback or a popup never retains the session in return.
struct DemoSession {
    item: Option<StatusItem>,
    main_window: Option<AnyWindowHandle>,
    // A reserved generation has no handle until deferred creation completes.
    popup: Option<(u64, Option<AnyWindowHandle>)>,
    generation: u64,
    count: u32,
    started_at: Option<Instant>,
    notice: Option<SharedString>,
    startup_evidence: SharedString,
}

impl DemoSession {
    fn new(startup_evidence: SharedString, cx: &mut Context<Self>) -> Self {
        let owner = cx.weak_entity();
        let result = StatusItem::new(
            StatusItemOptions::new(status_image(false), "Status Item Example: idle")
                .with_tooltip("Status Item Example — idle"),
            cx,
            move |cx| {
                // A queued native activation is harmless after the owner drops.
                if let Some(owner) = owner.upgrade() {
                    owner.update(cx, |session, cx| session.toggle_popup(cx));
                }
            },
        );
        let (item, notice) = match result {
            Ok(item) => (Some(item), None),
            Err(error) => (None, Some(error.to_string().into())),
        };
        Self {
            item,
            main_window: None,
            popup: None,
            generation: 0,
            count: 0,
            started_at: None,
            notice,
            startup_evidence,
        }
    }

    fn toggle_popup(&mut self, cx: &mut Context<Self>) {
        if self.popup.is_some() {
            self.close_popup(cx);
            return;
        }
        let Some(anchor) = self.item.as_ref().and_then(StatusItem::anchor) else {
            self.notice = Some("The menu bar item has no visible display anchor.".into());
            cx.notify();
            return;
        };
        self.generation += 1;
        let generation = self.generation;
        self.popup = Some((generation, None));
        let owner = cx.weak_entity();
        // This is native window geometry; content spacing remains rem-based.
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(
                anchor.popup_bounds(size(px(352.), px(420.))),
            )),
            display_id: Some(anchor.display_id()),
            kind: WindowKind::PopUp,
            titlebar: None,
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            focus: true,
            ..Default::default()
        };
        // open_window can synchronously render its content. That content reads
        // this session, so the mutable session lease must end before opening.
        cx.defer(move |cx| open_popup(owner, generation, anchor, options, cx));
        cx.notify();
    }

    fn close_popup(&mut self, cx: &mut Context<Self>) {
        let Some((_, handle)) = self.popup.take() else {
            return;
        };
        if let Some(item) = &mut self.item {
            item.set_selected(false);
        }
        if let Some(handle) = handle {
            // Native close/release callbacks may also read the shared session.
            cx.defer(move |cx| {
                if let Err(error) = handle.update(cx, |_, window, _| window.remove_window()) {
                    eprintln!("Popup was already closed: {error}");
                }
            });
        }
        cx.notify();
    }

    fn popup_released(&mut self, generation: u64, cx: &mut Context<Self>) {
        // A delayed release from an older popup must not unselect a new one.
        if self.popup.is_some_and(|(current, _)| current == generation) {
            self.popup = None;
            if let Some(item) = &mut self.item {
                item.set_selected(false);
            }
            cx.notify();
        }
    }

    fn toggle_demo(&mut self, cx: &mut Context<Self>) {
        let running = self.started_at.is_none();
        if let Some(item) = &mut self.item {
            if let Err(error) = item.set_image(status_image(running)) {
                self.notice = Some(format!("Could not update status image: {error}").into());
                cx.notify();
                return;
            }
            let text = if running {
                "Status Item Example: generated audio demo running"
            } else {
                "Status Item Example: idle"
            };
            item.set_accessible_name(text);
            item.set_tooltip(Some(text));
        }
        self.started_at = running.then(Instant::now);
        cx.notify();
    }

    fn increment(&mut self, cx: &mut Context<Self>) {
        self.count = self.count.saturating_add(1);
        cx.notify();
    }

    fn open_main(&mut self, cx: &mut Context<Self>) {
        self.close_popup(cx);
        if let Some(handle) = self.main_window {
            let owner = cx.weak_entity();
            // Activation can redraw the main window. Run after close_popup's
            // deferred close and outside this session's mutable lease.
            cx.defer(move |cx| {
                let result = handle.update(cx, |_, window, cx| {
                    cx.activate(true);
                    window.activate_window();
                });
                if let Err(error) = result {
                    if let Some(owner) = owner.upgrade() {
                        owner.update(cx, |session, cx| {
                            session.notice =
                                Some(format!("Could not activate main window: {error}").into());
                            cx.notify();
                        });
                    }
                }
            });
        }
    }
}

fn open_popup(
    owner: WeakEntity<DemoSession>,
    generation: u64,
    anchor: StatusItemAnchor,
    options: WindowOptions,
    cx: &mut App,
) {
    let Some(session) = owner.upgrade() else {
        return;
    };
    if !session
        .read(cx)
        .popup
        .is_some_and(|(current, _)| current == generation)
    {
        return;
    }
    // No session update/lease spans this call or the first synchronous render.
    let opened = gpui_kit::open_window(options, cx, move |window, cx| {
        window.set_window_title("Status Item Popup");
        cx.new(|cx| DemoPopup::new(owner, generation, anchor, window, cx))
    });
    match opened {
        Ok((handle, _view)) => {
            let adopted = session.update(cx, |session, cx| {
                if !session
                    .popup
                    .is_some_and(|(current, _)| current == generation)
                {
                    return false;
                }
                session.popup = Some((generation, Some(handle)));
                session.notice = None;
                if let Some(item) = &mut session.item {
                    item.set_selected(true);
                }
                cx.notify();
                true
            });
            if !adopted {
                if let Err(error) = handle.update(cx, |_, window, _| window.remove_window()) {
                    eprintln!("Unclaimed popup was already closed: {error}");
                }
            }
        }
        Err(error) => session.update(cx, |session, cx| {
            if session
                .popup
                .is_some_and(|(current, _)| current == generation)
            {
                session.popup = None;
                session.notice = Some(format!("Could not open popup: {error}").into());
                cx.notify();
            }
        }),
    }
}

fn status_image(running: bool) -> StatusItemImage {
    let bytes: &'static [u8] = if running {
        include_bytes!("../fixtures/running.png")
    } else {
        include_bytes!("../fixtures/idle.png")
    };
    // An 18-point native image backed by a 54-pixel template asset.
    StatusItemImage::new(bytes, size(px(18.), px(18.)))
}

struct MainWindow {
    session: Entity<DemoSession>,
    _subscription: Subscription,
}

impl MainWindow {
    fn new(session: Entity<DemoSession>, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe(&session, |_, _, cx| cx.notify());
        Self {
            session,
            _subscription: subscription,
        }
    }
}

impl Render for MainWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let session = self.session.read(cx);
        let running = session.started_at.is_some();
        let available = session.item.is_some();
        v_flex()
            .size_full()
            .p_6()
            .gap_4()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(div().text_xl().font_semibold().child("Status item example"))
            .child("Use the waveform icon in the macOS menu bar to open the popup.")
            .child(
                div().text_sm().text_color(cx.theme().muted_foreground)
                    .child("The meters use generated values. This example opens no audio devices and changes no login settings."),
            )
            .child(format!("Shared counter: {}", session.count))
            .child(
                h_flex().gap_2()
                    .child(Button::new("main-demo").label(if running { "Stop demo" } else { "Start demo" })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.session.update(cx, |session, cx| session.toggle_demo(cx));
                        })))
                    .child(Button::new("main-increment").label("Increment counter")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.session.update(cx, |session, cx| session.increment(cx));
                        })))
                    .child(Button::new("main-popup")
                        .label(if session.popup.is_some() { "Close popup" } else { "Open popup" })
                        .disabled(!available)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.session.update(cx, |session, cx| session.toggle_popup(cx));
                        }))),
            )
            .children(session.notice.clone().map(|notice| div().text_sm().child(notice)))
            .child(div().text_xs().text_color(cx.theme().muted_foreground)
                .child(session.startup_evidence.clone()))
            .child(
                div().text_sm().text_color(cx.theme().muted_foreground)
                    .child("Closing the main window quits this prototype. Per-window hide and reopen are not supported by this pinned GPUI API yet."),
            )
            .child(Button::new("main-quit").label("Quit example").on_click(|_, _, cx| cx.quit()))
    }
}

struct DemoPopup {
    owner: WeakEntity<DemoSession>,
    focus: FocusHandle,
    anchor: StatusItemAnchor,
    opened_at: Instant,
    levels: [f32; 2],
    has_been_active: bool,
    _subscriptions: Vec<Subscription>,
    _updates: Task<()>,
}

impl DemoPopup {
    fn new(
        owner: WeakEntity<DemoSession>,
        generation: u64,
        anchor: StatusItemAnchor,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let mut subscriptions = vec![cx.observe_window_activation(window, |this, window, _| {
            if window.is_window_active() {
                this.has_been_active = true;
            } else if this.has_been_active {
                window.remove_window();
            }
        })];
        if let Some(session) = owner.upgrade() {
            subscriptions.push(cx.observe(&session, |_, _, cx| cx.notify()));
        }
        let release_owner = owner.clone();
        subscriptions.push(cx.on_release(move |_, cx| {
            if let Some(owner) = release_owner.upgrade() {
                owner.update(cx, |session, cx| session.popup_released(generation, cx));
            }
        }));
        let executor = cx.background_executor().clone();
        let updates = cx.spawn_in(window, async move |this, cx| {
            loop {
                executor.timer(Duration::from_millis(100)).await;
                let result =
                    cx.update(|window, cx| this.update(cx, |this, cx| this.sample(window, cx)));
                if !matches!(result, Ok(Ok(true))) {
                    break;
                }
            }
        });
        Self {
            owner,
            focus,
            anchor,
            opened_at: Instant::now(),
            levels: [0., 0.],
            has_been_active: window.is_window_active(),
            _subscriptions: subscriptions,
            _updates: updates,
        }
    }

    fn sample(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(owner) = self.owner.upgrade() else {
            window.remove_window();
            return false;
        };
        let session = owner.read(cx);
        let anchor = session.item.as_ref().and_then(StatusItem::anchor);
        // A menu bar move, display removal, scale change or missing anchor
        // dismisses the popup. A fresh activation will use fresh geometry.
        if anchor != Some(self.anchor) {
            window.remove_window();
            return false;
        }
        let running = session.started_at.is_some();
        let levels = if running {
            let seconds = self.opened_at.elapsed().as_secs_f32();
            [
                48. + (seconds * 3.1).sin() * 35.,
                38. + (seconds * 2.3 + 1.).sin() * 31.,
            ]
        } else {
            [0., 0.]
        };
        if running || self.levels != levels {
            self.levels = levels;
            cx.notify();
        }
        true
    }

    fn dismiss(&mut self, _: &DismissPopup, window: &mut Window, _: &mut Context<Self>) {
        window.remove_window();
    }
}

impl Render for DemoPopup {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = self.owner.upgrade();
        let session = owner.as_ref().map(|owner| owner.read(cx));
        let running = session.is_some_and(|session| session.started_at.is_some());
        let elapsed = session
            .and_then(|session| session.started_at)
            .map_or(0, |at| at.elapsed().as_secs());
        let count = session.map_or(0, |session| session.count);
        let levels = if running { self.levels } else { [0., 0.] };
        let status = if running { "Demo running" } else { "Idle" };
        let content = v_flex()
            .p_4()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        div()
                            .id("demo-status")
                            .role(accesskit::Role::Status)
                            .aria_label(status)
                            .a11y_synthetic_children(|builder| {
                                // Keep the same mounted node while the semantic
                                // state changes. Timer/meter values are siblings.
                                builder.parent_node().set_live(accesskit::Live::Polite);
                            })
                            .font_semibold()
                            .child(status),
                    )
                    .child(format!("{:02}:{:02}", elapsed / 60, elapsed % 60)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Generated audio levels · no recording"),
            )
            .child(meter("microphone-level", "Microphone", levels[0], cx))
            .child(meter("system-level", "System audio", levels[1], cx))
            .child(
                Button::new("popup-demo")
                    .label(if running { "Stop demo" } else { "Start demo" })
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(owner) = this.owner.upgrade() {
                            owner.update(cx, |session, cx| session.toggle_demo(cx));
                        }
                    })),
            )
            .child(
                h_flex()
                    .justify_between()
                    .child(format!("Counter: {count}"))
                    .child(
                        Button::new("popup-increment")
                            .label("Increment counter")
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(owner) = this.owner.upgrade() {
                                    owner.update(cx, |session, cx| session.increment(cx));
                                }
                            })),
                    ),
            )
            .child(
                Button::new("popup-open-main")
                    .label("Open main window")
                    .on_click(cx.listener(|this, _, _, cx| {
                        let owner = this.owner.clone();
                        // Closing the current window must happen after its update.
                        cx.defer(move |cx| {
                            if let Some(owner) = owner.upgrade() {
                                owner.update(cx, |session, cx| session.open_main(cx));
                            }
                        });
                    })),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Escape, an outside click, or the menu bar icon closes this popup."),
            );
        div()
            .id("status-popup")
            .key_context("StatusItemPopup")
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::dismiss))
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(content)
            .overflow_y_scrollbar()
    }
}

fn meter(id: &'static str, label: &'static str, level: f32, cx: &App) -> impl IntoElement {
    v_flex().gap_1().child(div().text_sm().child(label)).child(
        div()
            .id(id)
            .role(accesskit::Role::Meter)
            .aria_label(format!("Generated {label} level"))
            .aria_numeric_value(f64::from(level))
            .aria_min_numeric_value(0.)
            .aria_max_numeric_value(100.)
            .aria_value(format!("{level:.0} percent"))
            .w_full()
            .h_2()
            .rounded_full()
            .bg(cx.theme().muted)
            .child(
                div()
                    .h_full()
                    .w(relative(level / 100.))
                    .rounded_full()
                    .bg(cx.theme().primary),
            ),
    )
}

fn main() {
    gpui_kit::application().run(|cx| {
        // Capture the launch event synchronously, before async work can outlive
        // it. These calls query state; they never register or unregister the app.
        let launch_reason = current_launch_reason();
        let login_status = LoginItem::main_app().and_then(|item| item.status());
        let startup_evidence: SharedString =
            format!("Launch reason: {launch_reason:?}. Login item (read-only): {login_status:?}.")
                .into();
        gpui_kit::init(cx);
        cx.bind_keys([
            KeyBinding::new("secondary-q", Quit, None),
            KeyBinding::new("escape", DismissPopup, Some("StatusItemPopup")),
        ]);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.set_menus([Menu::new("Status Item Example")
            .items([MenuItem::action("Quit Status Item Example", Quit)])]);
        let session = cx.new(|cx| DemoSession::new(startup_evidence, cx));
        let view_session = session.clone();
        // Native window geometry is an intentional pixel boundary.
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(620.), px(460.)),
                cx,
            ))),
            ..Default::default()
        };
        let (main_window, _view) = gpui_kit::open_window(options, cx, |window, cx| {
            window.set_window_title("Status Item Example");
            cx.new(|cx| MainWindow::new(view_session, cx))
        })
        .expect("the example must open its main window");
        session.update(cx, |session, _| session.main_window = Some(main_window));
        cx.on_window_closed(move |cx, id| {
            if id == main_window.window_id() {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
}
