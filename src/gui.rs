//! eframe/egui graphical front-end.

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::Duration;

use eframe::egui;

use crate::args::{Action, Lang};
use crate::i18n::{self, Messages};
use crate::ui::{self, LogEvent};

pub fn run() -> eframe::Result<()> {
	let icon = eframe::icon_data::from_png_bytes(
		include_bytes!("../assets/project446.png"),
	)
	.expect("failed to load project446 icon");

	let opts = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default()
			.with_inner_size([960.0, 720.0])
			.with_min_inner_size([720.0, 520.0])
			.with_title("Project446 Fixer")
			.with_icon(icon),
		..Default::default()
	};
    eframe::run_native(
        "CS:GO Legacy Fixer",
        opts,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}

// ---------------------------------------------------------------------------
// Log line parsing (ANSI → colored spans)
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Span {
    text: String,
    color: Option<egui::Color32>,
    bold: bool,
}

#[derive(Clone)]
struct LogLine {
    spans: Vec<Span>,
}

fn parse_ansi(line: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut color: Option<egui::Color32> = None;
    let mut bold = false;
    let mut chars = line.chars().peekable();

    let flush = |spans: &mut Vec<Span>,
                 current: &mut String,
                 color: Option<egui::Color32>,
                 bold: bool| {
        if !current.is_empty() {
            spans.push(Span {
                text: std::mem::take(current),
                color,
                bold,
            });
        }
    };

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            flush(&mut spans, &mut current, color, bold);
            if chars.peek() == Some(&'[') {
                chars.next();
                let mut code = String::new();
                while let Some(c2) = chars.next() {
                    if c2.is_ascii_alphabetic() {
                        break;
                    }
                    code.push(c2);
                }
                match code.as_str() {
                    "" | "0" => {
                        color = None;
                        bold = false;
                    }
                    "1" => bold = true,
                    "90" => color = Some(egui::Color32::from_rgb(150, 150, 150)),
                    "91" => color = Some(egui::Color32::from_rgb(255, 110, 110)),
                    "92" => color = Some(egui::Color32::from_rgb(120, 220, 120)),
                    "93" => color = Some(egui::Color32::from_rgb(240, 215, 110)),
                    "94" => color = Some(egui::Color32::from_rgb(130, 175, 255)),
                    "96" => color = Some(egui::Color32::from_rgb(110, 220, 220)),
                    _ => {}
                }
            }
        } else {
            current.push(c);
        }
    }
    flush(&mut spans, &mut current, color, bold);
    spans
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

struct App {
    lang: Lang,
    path_input: String,
    log: Vec<LogLine>,
    log_rx: Option<Receiver<LogEvent>>,
    busy: bool,
    awaiting_continue: bool,
    progress: Option<(u64, u64)>,
    pending_close: bool,
}

impl App {
    fn new() -> Self {
        let lang = crate::win::registry::read_string("Language")
            .and_then(|s| match s.as_str() {
                "2" => Some(Lang::Ru),
                "1" => Some(Lang::En),
                _ => None,
            })
            .unwrap_or(Lang::En);

        let path_input = crate::paths::auto_detect()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();

        Self {
            lang,
            path_input,
            log: Vec::new(),
            log_rx: None,
            busy: false,
            awaiting_continue: false,
            progress: None,
            pending_close: false,
        }
    }

    fn msgs(&self) -> &'static Messages {
        i18n::messages(self.lang)
    }

    fn start_worker<F: FnOnce() + Send + 'static>(&mut self, f: F) {
        let (tx, rx) = channel();
        ui::set_gui(tx);
        ui::reset_cancel();
        self.log_rx = Some(rx);
        self.log.clear();
        self.busy = true;
        self.awaiting_continue = false;
        self.progress = None;
        std::thread::spawn(move || {
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            if let Err(p) = res {
                let msg = if let Some(s) = p.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = p.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "worker panicked".to_string()
                };
                ui::say_line(format_args!("[FATAL] {msg}"));
            }
            ui::finish();
        });
    }

    fn run_action(&mut self, action: Action) {
        let msgs = self.msgs();

        let raw = self.path_input.trim().to_string();
        if raw.is_empty() {
            self.log.push(LogLine { spans: parse_ansi(msgs.gui_path_empty) });
            return;
        }
        let path = PathBuf::from(&raw);
        if let Err(e) = crate::paths::validate(&path) {
            let line = format!("{}{}", msgs.gui_invalid_prefix, e);
            self.log.push(LogLine { spans: parse_ansi(&line) });
            return;
        }
        let _ = crate::win::registry::write_string("GamePath", &path.to_string_lossy());

        self.start_worker(move || {
            let res = crate::fixer::dispatch(action, &path, msgs, false);
            if let Err(e) = res {
                ui::say_line(format_args!("{}{:#}", msgs.gui_error_prefix, e));
            }
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ---- Graceful shutdown ----
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.busy {
                if crate::ui::request_cancel() {
                    crate::ui::say_line_str(self.msgs().cancelling);
                }
                self.pending_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }
        if self.pending_close && !self.busy {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        // ---- Drain log channel ----
        if let Some(rx) = self.log_rx.take() {
            let mut dirty = false;
            let mut done = false;
            loop {
                match rx.try_recv() {
                    Ok(LogEvent::Line(s)) => {
                        self.log.push(LogLine {
                            spans: parse_ansi(&s),
                        });

                        const MAX_LOG_LINES: usize = 10_000;

                        if self.log.len() > MAX_LOG_LINES {
                            let excess = self.log.len() - MAX_LOG_LINES;
                            self.log.drain(..excess);
                        }

                        dirty = true;
                    }
                    Ok(LogEvent::Progress { done: done_b, total }) => {
                        if total == 0 {
                            self.progress = None;
                        } else {
                            self.progress = Some((done_b, total));
                        }
                        dirty = true;
                    }
                    Ok(LogEvent::Pause) => {
                        self.awaiting_continue = true;

                        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                            egui::UserAttentionType::Critical,
                        ));

                        crate::win::process::beep();

                        dirty = true;
                    }
                    Ok(LogEvent::Done) => {
                        done = true;
                        break;
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        done = true;
                        break;
                    }
                }
            }
            if done {
                self.busy = false;
                self.progress = None;
                ui::unset_gui();
            } else {
                self.log_rx = Some(rx);
            }
            if dirty {
                ctx.request_repaint();
            }
            if self.busy {
                ctx.request_repaint_after(Duration::from_millis(200));
            }
        }

        // ---- Top: language ----
        let msgs = self.msgs();
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(msgs.gui_language_label);
                let mut l = self.lang;
                ui.selectable_value(&mut l, Lang::En, "English");
                ui.selectable_value(&mut l, Lang::Ru, "Русский");
                if l != self.lang {
                    self.lang = l;
                    let v = match l {
                        Lang::Ru => "2",
                        Lang::En => "1",
                    };
                    let _ = crate::win::registry::write_string("Language", v);
                }
            });
            ui.add_space(4.0);
        });

        // ---- Bottom: log ----
        egui::TopBottomPanel::bottom("log")
            .resizable(true)
            .default_height(300.0)
            .min_height(120.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.heading(self.msgs().gui_log_heading);
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in &self.log {
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.0;
                                for span in &line.spans {
                                    let mut text =
                                        egui::RichText::new(&span.text).monospace();
                                    if let Some(c) = span.color {
                                        text = text.color(c);
                                    }
                                    if span.bold {
                                        text = text.strong();
                                    }
                                    ui.label(text);
                                }
                            });
                        }
                    });
            });

        // ---- Center: controls ----
        egui::CentralPanel::default().show(ctx, |ui| {
            let msgs = self.msgs();

            ui.add_space(8.0);
            ui.heading(msgs.title);
            ui.separator();
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(msgs.gui_folder_label);
                let browse_w = 110.0;
                let w = (ui.available_width() - browse_w).max(200.0);
                ui.add_enabled(
                    !self.busy,
                    egui::TextEdit::singleline(&mut self.path_input).desired_width(w),
                );
                if ui
                    .add_enabled(!self.busy, egui::Button::new(msgs.gui_browse))
                    .clicked()
                {
                    if let Some(p) = ui::pick_folder(msgs.psdesc) {
                        self.path_input = p;
                    }
                }
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            let enabled = !self.busy;
            let btn_size = egui::vec2(ui.available_width(), 36.0);

            let r1 = ui
                .add_enabled_ui(enabled, |ui| {
                    ui.add_sized(btn_size, egui::Button::new(format!("1. {}", msgs.menu1)))
                })
                .inner;
            if r1.on_hover_text(msgs.menu1).clicked() {
                self.run_action(Action::Update);
            }

            ui.add_space(6.0);

            let r2 = ui
                .add_enabled_ui(enabled, |ui| {
                    ui.add_sized(btn_size, egui::Button::new(format!("2. {}", msgs.menu2)))
                })
                .inner;
            if r2.on_hover_text(msgs.menu2).clicked() {
                self.run_action(Action::Icons);
            }

            ui.add_space(6.0);

            let r3 = ui
                .add_enabled_ui(enabled, |ui| {
                    ui.add_sized(btn_size, egui::Button::new(format!("3. {}", msgs.menu3)))
                })
                .inner;
            if r3.on_hover_text(msgs.menu3).clicked() {
                self.run_action(Action::Infinite);
            }

            ui.add_space(6.0);

            let r4 = ui
                .add_enabled_ui(enabled, |ui| {
                    ui.add_sized(btn_size, egui::Button::new(format!("4. {}", msgs.menu4)))
                })
                .inner;
            if r4.on_hover_text(msgs.menu4).clicked() {
                self.run_action(Action::Validate);
            }

            ui.add_space(18.0);

            ui.horizontal(|ui| {
                if ui
                    .add_enabled(enabled, egui::Button::new(format!("6. {}", msgs.menu6)))
                    .clicked()
                {
                    let _ = crate::win::process::shell_open(
                        "https://t.me/reports_project446_bot",
                    );
                }
                if ui
                    .add_enabled(enabled, egui::Button::new(format!("7. {}", msgs.menu7)))
                    .clicked()
                {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            if self.busy && !self.awaiting_continue {
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(msgs.gui_working);
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if ui.button(msgs.cancel).clicked() {
                                if crate::ui::request_cancel() {
                                    crate::ui::say_line_str(msgs.cancelling);
                                }
                            }
                        },
                    );
                });
                if let Some((done, total)) = self.progress {
                    if total > 0 {
                        let frac = (done as f32 / total as f32).clamp(0.0, 1.0);
                        let mb_done = done as f64 / 1_048_576.0;
                        let mb_total = total as f64 / 1_048_576.0;
                        ui.add_space(6.0);
                        ui.add(
                            egui::ProgressBar::new(frac).text(format!(
                                "{:.1} / {:.1} MB",
                                mb_done, mb_total
                            )),
                        );
                    }
                }
            }
        });

        // ---- Inline "Press OK to continue" modal ----
        if self.awaiting_continue {
            let title = self.msgs().gui_continue_title;
            let cont = self.msgs().gui_continue;
            let mut clicked = false;
            egui::Window::new(title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.add_space(8.0);
                    if ui
                        .add_sized([220.0, 40.0], egui::Button::new(cont))
                        .clicked()
                    {
                        clicked = true;
                    }
                    ui.add_space(4.0);
                });
            if clicked {
                ui::resume();
                self.awaiting_continue = false;
            }
        }
    }
}