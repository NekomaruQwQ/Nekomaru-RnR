use std::time::Instant;

use egui::*;

use windows::Win32::Foundation::{HWND, SIZE};

use crate::native::*;
use crate::core::*;
pub const CHAR_CHECK_EMPTY: char = '\u{26ab}';
pub const CHAR_CHECK: char = '\u{2705}';
pub const CHAR_CROSS: char = '\u{00d7}';
pub const CHAR_WINDOW: char = '\u{1f5d6}';

pub struct WindowInfo {
    pub hwnd: HWND,
    pub name: String,
    pub size: Option<SIZE>,
    pub centered: Option<bool>,
}

impl WindowInfo {
    pub fn from_hwnd(hwnd: HWND) -> Self {
        Self {
            hwnd,
            name: get_window_text(hwnd),
            size: get_client_size(hwnd).ok(),
            centered: is_centered(hwnd),
        }
    }
}

pub struct App {
    windows: Vec<WindowInfo>,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| self.main_ui(ui));
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            windows: Self::enumerate_windows(),
        }
    }

    fn enumerate_windows() -> Vec<WindowInfo> {
        let start_time = Instant::now();
        let out =
            enumerate_windows()
                .inspect_err(|e| eprintln!("enumerate_windows() failed: {e}"))
                .unwrap_or_default()
                .into_iter()
                .filter(|&hwnd| is_active(hwnd))
                .map(WindowInfo::from_hwnd)
                .filter(|info| !info.name.is_empty())
                .collect();
        let time_elapsed_ms =
            start_time.elapsed().as_secs_f32() * 1000.0;
        println!("enumerate_windows() succeeded in {time_elapsed_ms:.2}ms");
        out
    }

    fn main_ui(&mut self, ui: &mut Ui) {
        ui.add_sized((ui.available_width(), 16.0), Button::new("REFRESH"))
            .clicked()
            .then(|| self.windows = Self::enumerate_windows());
        ui.group(|ui| {
            ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| {
                    Grid::new("windows")
                        .num_columns(1)
                        .spacing((12.0, 12.0))
                        .show(ui, |ui| {
                            for window in &mut self.windows {
                                ui.push_id(window.hwnd.0, |ui| {
                                    Self::item_ui(ui, window);
                                });
                                ui.end_row();
                            }
                        })
                })
            });
    }

    fn item_ui(ui: &mut Ui, window_info: &mut WindowInfo) {
        let &mut WindowInfo {
            hwnd,
            ref name,
            ref mut size,
            ref mut centered,
        } = window_info;

        Grid::new("grid").num_columns(1).show(ui, |ui| {
            ui.add(
                Label::new(
                    RichText::new(format!("{CHAR_WINDOW} {name}"))
                        .heading())
                    .truncate());
            ui.end_row();
            ui.horizontal(|ui| {
                if centered == &mut Some(true) {
                    ui.add_sized((80.0, 16.0), Label::new(format!("{CHAR_CHECK}centered")));
                } else {
                    ui.add_sized((80.0, 16.0), Button::new("CENTER"))
                        .clicked()
                        .then(|| {
                            let _ = center_to_screen(hwnd);
                            *centered = is_centered(hwnd);
                        });
                }

                egui::ComboBox::from_id_salt("size")
                    .width(ui.available_width().min(120.0))
                    .selected_text({
                        if let Some(&mut size@SIZE { cx, cy }) = size.as_mut() {
                            format!(
                                "{} {cx}{CHAR_CROSS}{cy}",
                                if is_known_resolution(size) {
                                    CHAR_CHECK
                                } else {
                                    CHAR_CHECK_EMPTY
                                })
                        } else {
                            "<unknown size>".to_owned()
                        }
                    })
                    .show_ui(ui, |ui| {
                        for &(name, arr) in RESOLUTION_GROUPS {
                            ui.add_sized(
                                (ui.available_width(), 0.0),
                                egui::Label::new(
                                    egui::RichText::new(format!("-{name}-  ")).weak()));
                            for resolution in arr {
                                ui.selectable_value(
                                    &mut size.as_ref()
                                        .copied()
                                        .unwrap_or_default(),
                                    *resolution,
                                    format!("{}{}{}", resolution.cx, CHAR_CROSS, resolution.cy))
                                    .clicked()
                                    .then(|| {
                                        let _ = resize_client(hwnd, resolution.cx, resolution.cy);
                                        *size = get_client_size(hwnd).ok();
                                    });
                            }
                            ui.label("");
                        }
                    });
            });
            ui.end_row();
        });
    }
}
