use gpui::{
    App, Application, AsyncApp, Bounds, Context, Entity, EventEmitter, PathBuilder, Pixels,
    SharedString, Subscription, TitlebarOptions, Window, WindowBounds, WindowOptions, canvas, div,
    point, prelude::*, px, rgb, size,
};
use smol::Timer;
use std::time::{Duration, Instant};

const START_ANGLE: f32 = -(std::f32::consts::PI / 2.0);

fn button(text: &str, on_click: impl Fn(&mut Window, &mut App) + 'static) -> impl IntoElement {
    div()
        .id(SharedString::from(text.to_string()))
        .flex_none()
        .px_4()
        .py_1()
        .bg(rgb(0x09090b))
        .hover(|s| s.bg(rgb(0x404040)))
        .active(|s| s.bg(rgb(0x363636)))
        .rounded_md()
        .cursor_pointer()
        .text_color(rgb(0xffffff))
        .child(text.to_string())
        .on_click(move |_, window, cx| on_click(window, cx))
}

struct TimerModel {
    duration: Duration,
    running: bool,
    end_time: Option<Instant>,
    remaining: Duration,
    settings: bool,
}

impl TimerModel {
    fn new(duration: Duration) -> Self {
        Self {
            duration,
            running: false,
            end_time: None,
            remaining: duration,
            settings: false,
        }
    }

    fn start(&mut self) {
        if self.remaining.is_zero() {
            self.remaining = self.duration;
        }
        self.end_time = Some(Instant::now() + self.remaining);
        self.running = true;
    }

    fn pause(&mut self) {
        if let Some(end) = self.end_time {
            let now = Instant::now();
            self.remaining = end.saturating_duration_since(now);
        }
        self.end_time = None;
        self.running = false;
    }

    fn toggle(&mut self) {
        if self.running {
            self.pause();
        } else {
            self.start();
        }
    }

    fn reset(&mut self) {
        self.remaining = self.duration;
        self.end_time = None;
        self.running = false;
    }

    // call from the periodic update loop to refresh remaining and handle finish
    fn update_tick(&mut self) {
        if let Some(end) = self.end_time {
            let now = Instant::now();
            if now >= end {
                // finished
                self.remaining = Duration::ZERO;
                self.end_time = None;
                self.running = false;
            } else {
                self.remaining = end - now;
            }
        }
    }

    fn smooth_progress(&self) -> f32 {
        let total = self.duration.as_secs_f32();
        if total <= 0.0 {
            return 1.0;
        }
        // If running, compute remaining relative to end_time
        if self.running {
            match self.end_time {
                Some(end) => {
                    let now = Instant::now();
                    let rem = end
                        .saturating_duration_since(now)
                        .as_secs_f32()
                        .clamp(0.0, total);
                    rem / total
                }
                _ => self.remaining.as_secs_f32().clamp(0.0, total) / total,
            }
        } else {
            // paused or stopped: exact remaining fraction
            self.remaining.as_secs_f32().clamp(0.0, total) / total
        }
    }

    // convenience getters for UI display in minutes/seconds
    fn seconds_total(&self) -> u64 {
        self.remaining.as_secs()
    }
}

impl EventEmitter<()> for TimerModel {}

struct TimerWindow {
    timer: Entity<TimerModel>,
    _sub: Subscription,
}

impl TimerWindow {
    fn new(cx: &mut Context<Self>) -> Self {
        let timer = cx.new(|_cx| TimerModel::new(Duration::from_secs(30)));
        let _sub = cx.subscribe(&timer, |_this, _model, _event: &(), cx| cx.notify());

        cx.spawn({
            let timer = timer.clone();
            async move |_, cx: &mut AsyncApp| {
                loop {
                    Timer::after(Duration::from_millis(100)).await;
                    let timer_update = timer.update(cx, |model, cx| {
                        // update remaining and handle finish
                        model.update_tick();
                        cx.emit(());
                    });

                    if timer_update.is_err() {
                        break;
                    }
                }
            }
        })
        .detach();

        Self { timer, _sub }
    }

    fn render_spinner(&self, remaining_frac: f32) -> impl IntoElement {
        let progress = 1.0 - remaining_frac.clamp(0.0, 1.0);
        canvas(
            move |_, _, _| {},
            move |bounds: Bounds<Pixels>, _data: (), window: &mut Window, _cx: &mut App| {
                if progress > 0.0 && progress < 1.0 {
                    window.request_animation_frame();
                }

                let mut builder = PathBuilder::stroke(px(8.0));

                let radius = px(120.0);
                let center = bounds.center();

                let end_angle = START_ANGLE + (2.0 * std::f32::consts::PI * (1.0 - progress));

                let point_on_circle = |angle: f32| {
                    point(
                        center.x + radius * angle.cos(),
                        center.y + radius * angle.sin(),
                    )
                };

                if (1.0 - progress) >= 1.0 - std::f32::EPSILON {
                    builder.move_to(point(center.x + radius, center.y));
                    builder.arc_to(
                        point(radius, radius),
                        px(0.0),
                        false,
                        false,
                        point(center.x - radius, center.y),
                    );
                    builder.arc_to(
                        point(radius, radius),
                        px(0.0),
                        false,
                        false,
                        point(center.x + radius, center.y),
                    );
                } else {
                    // partial arc
                    builder.move_to(point_on_circle(START_ANGLE));
                    builder.arc_to(
                        point(radius, radius),
                        px(0.0),
                        end_angle - START_ANGLE > std::f32::consts::PI,
                        true,
                        point_on_circle(end_angle),
                    );
                }

                let path = builder.build().unwrap();

                let color = rgb(0x00BFFF);
                window.paint_path(path, color);
            },
        )
        .size(px(240.))
        .absolute()
        .inset(px(0.))
    }
}

impl Render for TimerWindow {
    fn render(&mut self, _win: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let timer_data = self.timer.read(cx);
        let remaining_frac = timer_data.smooth_progress();
        let timer = self.timer.clone();

        if timer_data.settings == true {
            div()
                .flex()
                .size_full()
                .flex_col()
                .justify_center()
                .items_center()
                .gap_6()
                .bg(rgb(0x27272a))
                .text_3xl()
                .text_color(rgb(0xffffff))
                .child("Settings")
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap_6()
                        .text_base()
                        .child(button("Back", {
                            let timer = timer.clone();
                            move |_, cx| {
                                timer.update(cx, |model, _| {
                                    model.settings = false;
                                });
                            }
                        }))
                        .child(button("30s", {
                            let timer = timer.clone();
                            move |_, cx| {
                                timer.update(cx, |model, _| {
                                    model.duration = Duration::from_secs(30);
                                    model.reset();
                                });
                            }
                        }))
                        .child(button("5m", {
                            let timer = timer.clone();
                            move |_, cx| {
                                timer.update(cx, |model, _| {
                                    model.duration = Duration::from_secs(300);
                                    model.reset();
                                });
                            }
                        }))
                        .child(button("30m", {
                            let timer = timer.clone();
                            move |_, cx| {
                                timer.update(cx, |model, _| {
                                    model.duration = Duration::from_secs(1800);
                                    model.reset();
                                });
                            }
                        })),
                )
        } else {
            div()
                .bg(rgb(0x27272a))
                .size_full()
                .flex()
                .flex_col()
                .justify_center()
                .items_center()
                .gap_6()
                .child(
                    div()
                        .relative()
                        .size(px(240.))
                        .flex()
                        .justify_center()
                        .items_center()
                        .child(self.render_spinner(remaining_frac))
                        .child(
                            div()
                                .flex()
                                .justify_center()
                                .items_center()
                                .text_color(rgb(0xffffff))
                                .text_3xl()
                                .child(format!(
                                    "{:02}:{:02}",
                                    timer_data.seconds_total() / 60,
                                    timer_data.seconds_total() % 60
                                )),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .gap_4()
                        .justify_center()
                        .items_center()
                        .child(button(if timer_data.running { "Pause" } else { "Play" }, {
                            let timer = self.timer.clone();
                            move |_, cx| {
                                timer.update(cx, |model, cx| {
                                    model.toggle();
                                    cx.emit(());
                                });
                            }
                        }))
                        .child(button("Settings", {
                            let timer = self.timer.clone();
                            move |_, cx| {
                                timer.update(cx, |model, _| {
                                    model.settings = true;
                                });
                            }
                        }))
                        .child(button("Reset", {
                            let timer = self.timer.clone();
                            move |_, cx| {
                                timer.update(cx, |model, cx| {
                                    model.reset();
                                    cx.emit(());
                                });
                            }
                        })),
                )
        }
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(800.), px(800.0)), cx);
        let tb = TitlebarOptions {
            title: Some(SharedString::new_static("Timer")),
            ..Default::default()
        };
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(tb),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| TimerWindow::new(cx)),
        )
        .unwrap();
    });
}
