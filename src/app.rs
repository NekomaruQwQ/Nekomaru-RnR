use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use egui::*;

use itertools::Itertools as _;
use win32_version_info::VersionInfo;

use crate::native::*;
use crate::core::*;

pub const CHAR_CHECK_EMPTY: char = '\u{26ab}';
pub const CHAR_CHECK: char = '\u{2705}';
pub const CHAR_CROSS: char = '\u{00d7}';
pub const CHAR_WINDOW: char = '\u{1f5d6}';

#[derive(Debug, Default)]
pub struct App {
    windows: HashMap<Option<PathBuf>, Vec<WindowInfo>>,
    executables: HashMap<PathBuf, String>,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| self.main_ui(ui));
    }
}

impl App {
    pub fn new() -> Self {
        let mut app = Self::default();
        app.refresh();
        app
    }

    fn refresh(&mut self) {
        self.windows =
            Self::enumerate_windows()
                .into_iter()
                .into_group_map_by(|info| info.executable_path.clone());
        self.executables =
            self.windows
                .keys()
                .flatten()
                .filter_map(|path| {
                    let description =
                        VersionInfo::from_file(path)
                            .ok()?
                            .file_description;
                    Some((path.clone(), description))
                })
                .collect();
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
                .filter(|info| !info.window_text.is_empty())
                .filter(|info| !(
                    info.window_text == "Program Manager" &&
                    info.executable_path
                        .as_ref()
                        .and_then(|path| path.file_name())
                        .map(|name| name.to_string_lossy().to_lowercase())
                        .is_some_and(|name| name == "explorer.exe")))
                .collect();
        let time_elapsed_ms =
            start_time.elapsed().as_secs_f32() * 1000.0;
        println!("enumerate_windows() succeeded in {time_elapsed_ms:.2}ms");
        out
    }

    fn main_ui(&mut self, ui: &mut Ui) {
        ui.add_sized((ui.available_width(), 16.0), Button::new("REFRESH"))
            .clicked()
            .then(|| self.refresh());
        ui.separator();
        ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                for (executable_path, windows) in
                    self.windows
                        .iter_mut()
                        .sorted_by_key(|&(path, _)| path.clone()) {
                    let executable_name =
                        executable_path
                            .as_ref()
                            .and_then(|executable_path| {
                                self.executables
                                    .get(executable_path)
                                    .map(|s| Cow::Borrowed(s.as_str()))
                                    .or_else(|| {
                                        executable_path
                                            .file_name()
                                            .map(|name| name.to_string_lossy())
                                    })
                            })
                            .unwrap_or(Cow::Borrowed("<unknown>"));
                    let executable_path =
                        executable_path
                            .as_ref()
                            .map(|path| path.to_string_lossy())
                            .unwrap_or(Cow::Borrowed("<unknown>"));
                    ui.push_id(executable_path.as_ref(), |ui| {
                        Self::group_ui(
                            ui,
                            executable_name.as_ref(),
                            executable_path.as_ref(),
                            windows);
                    });
                    ui.add_space(8.0);
                }
            });
    }

    fn group_ui(
        ui: &mut Ui,
        executable_name: &str,
        executable_path: &str,
        windows: &mut Vec<WindowInfo>) {
        ui.heading(executable_name);
        ui.add(Label::new(RichText::new(executable_path).weak()).truncate());
        ui.add_space(4.0);
        ui.group(|ui| {
            ui.set_width(
                ui.available_width() -
                ui.style().spacing.item_spacing.x);
            for window in windows {
                ui.push_id(window.hwnd.0, |ui| Self::item_ui(ui, window));
                ui.add_space(2.0);
            }
        });
    }

    fn item_ui(ui: &mut Ui, window: &mut WindowInfo) {
        ui.horizontal(|ui| {
            ui.label(CHAR_WINDOW.to_string());
            ui.add(Label::new(&window.window_text).truncate());
        });

        ui.horizontal(|ui| {
            if window.is_centered == Some(true) {
                ui.add_sized((80.0, 16.0), Label::new(format!("{CHAR_CHECK}centered")));
            } else {
                ui.add_sized((80.0, 16.0), Button::new("CENTER"))
                    .clicked()
                    .then(|| {
                        if let Err(err) = center_to_screen(window.hwnd) {
                            eprintln!("failed to center window: {err}");
                        } else {
                            window.refresh();
                        }
                    });
            }

            let width = window.client_size.map_or(0, |size| size.width);
            let height = window.client_size.map_or(0, |size| size.height);

            egui::ComboBox::from_id_salt("size")
                .width(ui.available_width().min(120.0))
                .selected_text({
                    if !(width == 0 && height == 0) {
                        format!(
                            "{} {width}x{height}",
                            if is_known_resolution(width, height) {
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
                                &mut format!("{}x{}", resolution.cx, resolution.cy),
                                format!("{width}x{height}"),
                                format!("{}{}{}", resolution.cx, CHAR_CROSS, resolution.cy))
                                .clicked()
                                .then(|| {
                                    let _ = resize_client(window.hwnd, resolution.cx, resolution.cy);
                                    window.refresh();
                                });
                        }
                        ui.label("");
                    }
                });
        });
    }
}
