//! eframe/egui graphical front-end.

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use eframe::egui;

use crate::args::{Action, Lang};
use crate::i18n::{self, Messages};
use crate::ui::{self, LogEvent};

// ---------------------------------------------------------------------------
// Window geometry persistence (пункт 8)
// ---------------------------------------------------------------------------

const DEFAULT_WINDOW_SIZE: [f32; 2] = [960.0, 720.0];
const MIN_WINDOW_SIZE:     [f32; 2] = [720.0, 520.0];

fn read_window_size() -> [f32; 2] {
    let Some(s) = crate::win::registry::read_string("WindowSize") else {
        return DEFAULT_WINDOW_SIZE;
    };
    let mut it = s.split('x');
    let (Some(w), Some(h)) = (it.next(), it.next()) else { return DEFAULT_WINDOW_SIZE };
    let (Ok(w), Ok(h)) = (w.parse::<f32>(), h.parse::<f32>()) else { return DEFAULT_WINDOW_SIZE };
    if w < MIN_WINDOW_SIZE[0] || h < MIN_WINDOW_SIZE[1] {
        return DEFAULT_WINDOW_SIZE;
    }
    [w, h]
}

fn write_window_size(size: [f32; 2]) {
    let s = format!("{}x{}", size[0].round() as u32, size[1].round() as u32);
    let _ = crate::win::registry::write_string("WindowSize", &s);
}

pub fn run(initial_path: std::path::PathBuf) -> eframe::Result<()> {
    let icon = eframe::icon_data::from_png_bytes(
        include_bytes!("../assets/project446.png"),
    )
    .expect("failed to load project446 icon");

    let title = format!("Project446 Fixer v{}", env!("CARGO_PKG_VERSION"));

    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(read_window_size())
            .with_min_inner_size(MIN_WINDOW_SIZE)
            .with_title(title)
            .with_icon(icon),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "CS:GO Legacy Fixer",
        opts,
        Box::new(move |_cc| Ok(Box::new(App::new(initial_path)))),
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

impl LogLine {
    fn to_plain(&self) -> String {
        let mut s = String::new();
        for sp in &self.spans { s.push_str(&sp.text); }
        s
    }
}

pub(crate) fn parse_ansi(line: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut color: Option<egui::Color32> = None;
    let mut bold = false;
    let mut chars = line.chars().peekable();

    let flush = |spans: &mut Vec<Span>, current: &mut String,
                 color: Option<egui::Color32>, bold: bool| {
        if !current.is_empty() {
            spans.push(Span { text: std::mem::take(current), color, bold });
        }
    };

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            flush(&mut spans, &mut current, color, bold);
            if chars.peek() == Some(&'[') {
                chars.next();
                let mut code = String::new();
                while let Some(c2) = chars.next() {
                    if c2.is_ascii_alphabetic() { break; }
                    code.push(c2);
                }
                match code.as_str() {
                    "" | "0" => { color = None; bold = false; }
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

fn format_eta(secs: u64) -> String {
    if secs < 60 { format!("{}s left", secs) }
    else if secs < 3600 { format!("{}m {}s left", secs / 60, secs % 60) }
    else { format!("{}h {}m left", secs / 3600, (secs % 3600) / 60) }
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
    continue_ready_at: Option<Instant>,
    progress: Option<(u64, u64)>,
    progress_start: Option<(Instant, u64)>,
    pending_close: bool,

    // Пункт 8: debounce для сохранения размера окна.
    last_saved_size: Option<[f32; 2]>,
    next_size_save_at: Option<Instant>,

    // Пункт 5: список бэкапов + раскрыт ли диалог.
    backups: Vec<crate::fixer::restore::BackupEntry>,
    restore_open: bool,
}

impl App {
    fn new(initial_path: std::path::PathBuf) -> Self {
        let lang = crate::win::registry::read_string("Language")
            .and_then(|s| match s.as_str() {
                "2" => Some(Lang::Ru),
                "1" => Some(Lang::En),
                _ => None,
            })
            .unwrap_or_else(crate::win::detect_system_lang);

        let path_input = initial_path.to_string_lossy().into_owned();

        Self {
            lang,
            path_input,
            log: Vec::new(),
            log_rx: None,
            busy: false,
            awaiting_continue: false,
            continue_ready_at: None,
            progress: None,
            progress_start: None,
            pending_close: false,
            last_saved_size: None,
            next_size_save_at: None,
            backups: Vec::new(),
            restore_open: false,
        }
    }

    fn msgs(&self) -> &'static Messages { i18n::messages(self.lang) }

    fn start_worker<F: FnOnce() + Send + 'static>(&mut self, f: F) {
        let (tx, rx) = channel();
        ui::set_gui(tx);
        ui::reset_cancel();
        self.log_rx = Some(rx);
        self.log.clear();
        self.busy = true;
        self.awaiting_continue = false;
        self.continue_ready_at = None;
        self.progress = None;
        self.progress_start = None;
        std::thread::spawn(move || {
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            if let Err(p) = res {
                let msg = if let Some(s) = p.downcast_ref::<&str>() { (*s).to_string() }
                          else if let Some(s) = p.downcast_ref::<String>() { s.clone() }
                          else { "worker panicked".to_string() };
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

    /// Пункт 5: восстановление из конкретного бэкапа.
    fn run_restore(&mut self, entry: crate::fixer::restore::BackupEntry) {
        let msgs = self.msgs();
        let path = match crate::paths::validate(std::path::Path::new(&self.path_input)) {
            Ok(()) => PathBuf::from(self.path_input.trim()),
            Err(_) => {
                // hosts.bak не требует game path — но econ требует.
                if entry.kind == crate::fixer::restore::BackupKind::Hosts {
                    PathBuf::new()
                } else {
                    let line = format!("{}{}", msgs.gui_invalid_prefix, msgs.gui_path_empty);
                    self.log.push(LogLine { spans: parse_ansi(&line) });
                    return;
                }
            }
        };

        self.start_worker(move || {
            if let Err(e) = crate::fixer::restore::restore(&path, &entry, msgs) {
                ui::say_line(format_args!("{}{:#}", msgs.gui_error_prefix, e));
            }
        });
    }

    /// Пункт 6: сброс DNS на DHCP в фоновом воркере.
    fn run_dns_reset(&mut self) {
        let msgs = self.msgs();
        self.start_worker(move || {
            match crate::win::dns::reset_to_dhcp() {
                Ok(()) => ui::say_line_str(msgs.dns_reset_ok),
                Err(e) => ui::say_line(format_args!("{}{:#}", msgs.dns_reset_fail, e)),
            }
        });
    }

    fn save_log(&self) {
        let Some(path) = rfd::FileDialog::new()
            .set_file_name("csgo_legacy_fixer_log.txt")
            .save_file()
        else { return };
        let mut text = String::new();
        for line in &self.log {
            text.push_str(&line.to_plain());
            text.push('\n');
        }
        let _ = std::fs::write(&path, text);
    }

    fn open_backups(&self) {
        if let Some(dir) = crate::win::backup_root() {
            let _ = crate::win::process::shell_open_folder(&dir);
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

        // ---- Пункт 8: сохранить размер окна (debounce 2 c) ----
        let size = ctx.input(|i| i.screen_rect.size());
        let now = Instant::now();
        let due = self.next_size_save_at.map(|t| now >= t).unwrap_or(true);
        let changed = self.last_saved_size != Some([size.x, size.y]);
        if changed && due {
            write_window_size([size.x, size.y]);
            self.last_saved_size = Some([size.x, size.y]);
            self.next_size_save_at = Some(now + Duration::from_secs(2));
        }

        // ---- Drain log channel ----
        if let Some(rx) = self.log_rx.take() {
            let mut dirty = false;
            let mut done = false;
            loop {
                match rx.try_recv() {
                    Ok(LogEvent::Line(s)) => {
                        self.log.push(LogLine { spans: parse_ansi(&s) });
                        const MAX_LOG_LINES: usize = 10_000;
                        if self.log.len() > MAX_LOG_LINES {
                            let excess = self.log.len() - MAX_LOG_LINES;
                            self.log.drain(..excess);
                        }
                        dirty = true;
                    }
                    Ok(LogEvent::Progress { done: d, total }) => {
                        if total == 0 {
                            self.progress = None;
                            self.progress_start = None;
                        } else {
                            let was_idle = self.progress.map(|(p, _)| p == 0).unwrap_or(true);
                            self.progress = Some((d, total));
                            if was_idle || self.progress_start.is_none() {
                                self.progress_start = Some((Instant::now(), d));
                            }
                        }
                        dirty = true;
                    }
                    Ok(LogEvent::Pause) => {
                        self.awaiting_continue = true;
                        self.continue_ready_at = None;
                        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                            egui::UserAttentionType::Critical));
                        crate::win::process::beep();
                        dirty = true;
                    }
                    Ok(LogEvent::PauseDelayed { secs }) => {
                        self.awaiting_continue = true;
                        self.continue_ready_at =
                            Some(Instant::now() + Duration::from_secs(secs as u64));
                        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                            egui::UserAttentionType::Critical));
                        crate::win::process::beep();
                        dirty = true;
                    }
                    Ok(LogEvent::Done) => { done = true; break; }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => { done = true; break; }
                }
            }
            if done {
                self.busy = false;
                self.progress = None;
                self.progress_start = None;
                self.continue_ready_at = None;
                ui::unset_gui();
                // Обновим список бэкапов — после успешного fixer'а могли появиться новые.
                self.backups = crate::fixer::restore::list();
            } else {
                self.log_rx = Some(rx);
            }
            if dirty { ctx.request_repaint(); }
            if self.busy { ctx.request_repaint_after(Duration::from_millis(200)); }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let msgs = self.msgs();

        // ---- Top: language ----
        egui::Panel::top("top").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(msgs.gui_language_label);
                let mut l = self.lang;
                ui.selectable_value(&mut l, Lang::En, "English");
                ui.selectable_value(&mut l, Lang::Ru, "Русский");
                if l != self.lang {
                    self.lang = l;
                    let v = match l { Lang::Ru => "2", Lang::En => "1" };
                    let _ = crate::win::registry::write_string("Language", v);
                }
            });
            ui.add_space(4.0);
        });

        // ---- Bottom: log (пункт 7: виртуализация) ----
        egui::Panel::bottom("log")
            .resizable(true)
            .default_size(300.0)
            .min_size(120.0)
            .show(ui, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.heading(self.msgs().gui_log_heading);
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if ui.button(self.msgs().gui_save_log).clicked() {
                                self.save_log();
                            }
                        },
                    );
                });
                ui.separator();

                // show_rows: рендерим только видимые строки. Высота
                // строки = высота monospace-стиля; для многострочных
                // (wrapped) строк это даст небольшое переполнение, но
                // платим мы за это только на строках, которые реально
                // видны — 10k строк теперь не строят 10k Ui-объектов.
                let row_h = ui.text_style_height(&egui::TextStyle::Monospace);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show_rows(ui, row_h, self.log.len(), |ui, range| {
                        for i in range {
                            let line = &self.log[i];
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.0;
                                for span in &line.spans {
                                    let mut text =
                                        egui::RichText::new(&span.text).monospace();
                                    if let Some(c) = span.color { text = text.color(c); }
                                    if span.bold { text = text.strong(); }
                                    ui.label(text);
                                }
                            });
                        }
                    });
            });

        // ---- Center: controls ----
        egui::CentralPanel::default().show(ui, |ui| {
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
                if ui.add_enabled(!self.busy, egui::Button::new(msgs.gui_browse)).clicked() {
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

            let r1 = ui.add_enabled_ui(enabled, |ui| {
                ui.add_sized(btn_size, egui::Button::new(format!("1. {}", msgs.menu1)))
            }).inner;
            if r1.on_hover_text(msgs.menu1_tip).clicked() {
                self.run_action(Action::Update);
            }

            ui.add_space(6.0);
            let r2 = ui.add_enabled_ui(enabled, |ui| {
                ui.add_sized(btn_size, egui::Button::new(format!("2. {}", msgs.menu2)))
            }).inner;
            if r2.on_hover_text(msgs.menu2_tip).clicked() {
                self.run_action(Action::Icons);
            }

            ui.add_space(6.0);
            let r3 = ui.add_enabled_ui(enabled, |ui| {
                ui.add_sized(btn_size, egui::Button::new(format!("3. {}", msgs.menu3)))
            }).inner;
            if r3.on_hover_text(msgs.menu3_tip).clicked() {
                self.run_action(Action::Infinite);
            }

            ui.add_space(6.0);
            let r4 = ui.add_enabled_ui(enabled, |ui| {
                ui.add_sized(btn_size, egui::Button::new(format!("4. {}", msgs.menu4)))
            }).inner;
            if r4.on_hover_text(msgs.menu4_tip).clicked() {
                self.run_action(Action::Validate);
            }

            ui.add_space(18.0);

            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(enabled, egui::Button::new(format!("6. {}", msgs.menu6))).clicked() {
                    let _ = crate::win::process::shell_open("https://t.me/reports_project446_bot");
                }
                if ui.add_enabled(enabled, egui::Button::new(format!("7. {}", msgs.menu7))).clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if ui.button(msgs.gui_open_backups).clicked() {
                    self.open_backups();
                }
                if ui.button(msgs.gui_save_log).clicked() {
                    self.save_log();
                }
                // Пункт 5.
                if ui.add_enabled(!self.busy, egui::Button::new(msgs.restore_button)).clicked() {
                    self.backups = crate::fixer::restore::list();
                    self.restore_open = true;
                }
                // Пункт 6.
                if ui.add_enabled(!self.busy, egui::Button::new(msgs.dns_reset_button)).clicked() {
                    self.run_dns_reset();
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

                        let eta = self.progress_start.and_then(|(t0, d0)| {
                            let elapsed = t0.elapsed().as_secs_f64();
                            let delta = done.saturating_sub(d0);
                            if elapsed >= 1.0 && delta > 0 {
                                let speed = delta as f64 / elapsed;
                                let remain = (total.saturating_sub(done)) as f64 / speed;
                                if remain.is_finite() && remain < 86_400.0 {
                                    return Some(format_eta(remain as u64));
                                }
                            }
                            None
                        });

                        let text = match eta {
                            Some(e) => format!("{:.1} / {:.1} MB  ·  {}", mb_done, mb_total, e),
                            None => format!("{:.1} / {:.1} MB", mb_done, mb_total),
                        };

                        ui.add_space(6.0);
                        ui.add(egui::ProgressBar::new(frac).text(text));
                    }
                }
            }
        });

        // ---- Continue / Abort modal ----
        if self.awaiting_continue {
            let title = self.msgs().gui_continue_title;
            let cont = self.msgs().gui_continue;
            let cancel_label = self.msgs().cancel;
            let wait_label = self.msgs().gui_continue_wait;

            let remaining = self.continue_ready_at
                .map(|t| t.saturating_duration_since(Instant::now()));
            let button_enabled = remaining.map(|d| d.is_zero()).unwrap_or(true);

            let mut clicked_continue = false;
            let mut clicked_cancel = false;

            egui::Window::new(title)
                .collapsible(false)
                .resizable(false)
                .min_width(260.0)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.add_space(8.0);
                    let btn_width = ui.available_width();
                    if button_enabled {
                        if ui.add_sized([btn_width, 40.0], egui::Button::new(cont)).clicked() {
                            clicked_continue = true;
                        }
                    } else {
                        let secs = remaining.map(|d| d.as_secs_f32().ceil() as u32).unwrap_or(0);
                        let label = format!("{} ({}s)", wait_label, secs);
                        ui.add_enabled_ui(false, |ui| {
                            let _ = ui.add_sized([btn_width, 40.0], egui::Button::new(label));
                        });
                    }
                    ui.add_space(6.0);
                    if ui.add_sized([btn_width, 28.0], egui::Button::new(cancel_label)).clicked() {
                        clicked_cancel = true;
                    }
                    ui.add_space(4.0);
                });

            if clicked_continue {
                ui::resume();
                self.awaiting_continue = false;
                self.continue_ready_at = None;
            }
            if clicked_cancel {
                if crate::ui::request_cancel() {
                    crate::ui::say_line_str(self.msgs().cancelling);
                }
                ui::resume();
                self.awaiting_continue = false;
                self.continue_ready_at = None;
            }
            if !button_enabled {
                ui.ctx().request_repaint_after(Duration::from_millis(200));
            }
        }

        // ---- Restore-from-backup modal (пункт 5) ----
        if self.restore_open {
            let msgs = self.msgs();
            let title = msgs.restore_dialog_title;
            let empty = msgs.restore_empty;
            let cancel_label = msgs.cancel;
            let ok_label = msgs.restore_ok;

            let mut close = false;
            let mut chosen: Option<usize> = None;

            // Обращаемся к копии, чтобы не бороться с borrow checker внутри замыкания.
            let backups = self.backups.clone();

            egui::Window::new(title)
                .collapsible(false)
                .resizable(true)
                .min_width(420.0)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.add_space(6.0);
                    if backups.is_empty() {
                        ui.label(empty);
                    } else {
                        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                            for (i, b) in backups.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.monospace(&b.label);
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.button(ok_label).clicked() {
                                                chosen = Some(i);
                                            }
                                        },
                                    );
                                });
                            }
                        });
                    }
                    ui.add_space(6.0);
                    ui.separator();
                    if ui.button(cancel_label).clicked() {
                        close = true;
                    }
                });

            if let Some(i) = chosen {
                let entry = self.backups[i].clone();
                close = true;
                self.run_restore(entry);
            }
            if close {
                self.restore_open = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_ansi;

    #[test]
    fn ansi_plain_text_no_spans() {
        let spans = parse_ansi("hello world");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "hello world");
        assert!(spans[0].color.is_none());
        assert!(!spans[0].bold);
    }

    #[test]
    fn ansi_color_is_attached() {
        let spans = parse_ansi("\x1b[91mred\x1b[0m");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "red");
        assert!(spans[0].color.is_some());
    }

    #[test]
    fn ansi_reset_clears_state() {
        let spans = parse_ansi("\x1b[1;91mboth\x1b[0mplain");
        let plain = spans.last().unwrap();
        assert_eq!(plain.text, "plain");
        assert!(plain.color.is_none());
        assert!(!plain.bold);
    }
}
