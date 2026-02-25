mod app;
mod core;
mod native;

fn main() -> eframe::Result {
    eframe::run_native(
        "Percents", eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size((360.0, 540.0))
                .with_maximize_button(false),
            centered: true,
            ..Default::default()
        },
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            cc.egui_ctx.style_mut(|style| {
                style.interaction.selectable_labels = false;
            });
            Ok(Box::new(app::App::new()))
        }))
}
