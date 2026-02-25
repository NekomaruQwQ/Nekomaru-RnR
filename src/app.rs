use std::time::Instant;

use egui::*;

use windows::core::Result;
use windows::Win32::Foundation::{HWND, SIZE};

use crate::native::*;
use crate::core::*;

pub const CHAR_CHECK: char = '\u{2705}';
pub const CHAR_CROSS: char = '\u{00d7}';
pub const CHAR_WINDOW: char = '\u{1f5d6}';

pub struct WindowInfo {
    pub hwnd: HWND,
    pub name: String,
    pub size: SIZE,
    pub is_centered: bool,
}

impl WindowInfo {
    pub fn from_hwnd(hwnd: HWND) -> Result<Self> {
        Ok(Self {
            hwnd,
            name: get_window_text(hwnd),
            size: get_client_size(hwnd)?,
            is_centered: is_centered(hwnd)?,
        })
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
                .into_iter()
                .flatten()
                .filter(|&hwnd| is_active(hwnd))
                .flat_map(WindowInfo::from_hwnd)
                .filter(|info| !info.name.is_empty())
                .collect();
        let time_elapsed_ms =
            start_time.elapsed().as_secs_f32() * 1000.0;
        println!("enumerate_windows() in {time_elapsed_ms:.2}ms");
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
                                ui.push_id(
                                    window.hwnd.0,
                                    |ui| Self::item_ui(ui, window));
                                ui.end_row();
                            }
                        })
                })
            });
    }

    fn item_ui(
        ui: &mut Ui,
        &mut WindowInfo {
            hwnd,
            ref name,
            size,
            ref mut is_centered,
        }: &mut WindowInfo) {
        Grid::new("grid").num_columns(1).show(ui, |ui| {
            ui.add(
                Label::new(
                    RichText::new(format!("{CHAR_WINDOW} {name}"))
                        .heading())
                    .truncate());
            ui.end_row();
            ui.horizontal(|ui| {
                if *is_centered {
                    ui.add_sized((80.0, 16.0), egui::Label::new(format!("{CHAR_CHECK}centered")));
                } else if
                    ui.add_sized((80.0, 16.0), egui::Button::new("CENTER")).clicked() &&
                    center_to_screen(hwnd).is_ok() {
                    *is_centered = true;
                } else {}

                egui::ComboBox::from_id_salt("size")
                    .selected_text(format!(
                        "{}{}{}{}",
                        is_known_resolution(size)
                            .then_some(format!("{CHAR_CHECK} "))
                            .unwrap_or_default(),
                        size.cx,
                        CHAR_CROSS,
                        size.cy))
                    .show_ui(ui, |ui| {
                        for &(name, arr) in RESOLUTION_GROUPS {
                            ui.add_sized(
                                (ui.available_width(), 0.0),
                                egui::Label::new(
                                    egui::RichText::new(format!("-{name}-  ")).weak()));
                            for size in arr {
                                ui.selectable_value(
                                    &mut SIZE::default(),
                                    *size,
                                    format!("{}{}{}", size.cx, CHAR_CROSS, size.cy))
                                    .clicked()
                                    .then(|| {
                                        let _ = resize_client(hwnd, size.cx, size.cy);
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
