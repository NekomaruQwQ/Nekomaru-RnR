use std::collections::HashMap;
use std::path::PathBuf;

use egui::*;

use itertools::Itertools as _;

use crate::native::*;
use crate::core::*;

const CHAR_EMPTY: char = '\u{26ab}';
const CHAR_CHECK: char = '\u{2705}';
const CHAR_CROSS: char = '\u{00d7}';
const CHAR_WINDOW: char = '\u{1f5d6}';

pub struct App {
    windows: HashMap<Option<PathBuf>, Vec<WindowInfo>>,
    executables: HashMap<PathBuf, ExecutableInfo>,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| self.main_ui(ui));
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            executables: HashMap::new(),
        }
    }

    fn refresh_windows(&mut self) {
        self.windows =
            Self::enumerate_windows()
                .into_iter()
                .into_group_map_by(|info| info.executable_path.clone());
    }

    fn enumerate_windows() -> Vec<WindowInfo> {
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
            .collect()
    }

    fn main_ui(&mut self, ui: &mut Ui) {
        self.refresh_windows();

        ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                for (executable_path, windows) in
                    self.windows
                        .iter()
                        .sorted_by_key(|&(path, _)| path.clone()) {
                    Self::group_ui(
                        ui,
                        &mut self.executables,
                        executable_path.as_ref(),
                        windows);
                }
            });
    }

    fn group_ui(
        ui: &mut Ui,
        executables: &mut HashMap<PathBuf, ExecutableInfo>,
        my_path: Option<&PathBuf>,
        my_windows: &[WindowInfo]) {
        let my_info =
            my_path.map(|path| &*{
                executables
                    .entry(path.clone())
                    .or_insert_with(|| ExecutableInfo::from_path(path))
            });
        let my_display_name =
            my_info
                .and_then(|info| info.display_name.as_ref())
                .map(String::as_str)
                .unwrap_or("<unknown>");
        let my_display_path =
            my_info
                .map(|info| info.display_path.as_str())
                .unwrap_or("<unknown path>");
        ui.push_id(my_display_path, |ui| {
            ui.heading(my_display_name);
            ui.add(Label::new(RichText::new(my_display_path).weak()).truncate());
            ui.add_space(4.0);
            for window in my_windows {
                ui.push_id(window.hwnd.0, |ui| Self::item_ui(ui, window));
                ui.add_space(2.0);
            }
            ui.add_space(8.0);
        });
    }

    fn item_ui(ui: &mut Ui, window: &WindowInfo) {
        ui.horizontal(|ui| {
            ui.label(CHAR_WINDOW.to_string());
            match window.state {
                WindowState::Maximized => {
                    ui.add(Label::new(RichText::new("[max]").weak()));
                },
                WindowState::Minimized => {
                    ui.add(Label::new(RichText::new("[min]").weak()));
                },
                WindowState::Normal => {}
            }
            ui.add(Label::new(&window.window_text).truncate());
        });

        ui.horizontal(|ui| {
            if window.is_centered == Some(true) {
                ui.add_sized((80.0, 16.0), Label::new(format!("{CHAR_CHECK}centered")));
            } else {
                ui.add_sized((80.0, 16.0), Button::new("CENTER"))
                    .clicked()
                    .then(|| {
                        let result = match window.state {
                            WindowState::Normal =>
                                center_to_screen(window.hwnd),
                            WindowState::Maximized | WindowState::Minimized =>
                                center_restored_to_screen(window.hwnd),
                        };
                        if let Err(err) = result {
                            eprintln!("failed to center window: {err}");
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
                                CHAR_EMPTY
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
                                    let result = match window.state {
                                        WindowState::Normal =>
                                            resize_client(window.hwnd, resolution.cx, resolution.cy),
                                        WindowState::Maximized | WindowState::Minimized =>
                                            resize_restored_client(window.hwnd, resolution.cx, resolution.cy),
                                    };
                                    if let Err(err) = result {
                                        eprintln!("failed to resize window: {err}");
                                    }
                                });
                        }
                        ui.label("");
                    }
                });
        });
    }
}
