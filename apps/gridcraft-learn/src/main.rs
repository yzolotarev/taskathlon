#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use gridcraft_learn::LearningApp;
mod native;

struct Host(LearningApp);
impl eframe::App for Host {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.0.logic(ctx);
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.0.ui(ui);
    }
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.0.raw_input_hook(raw);
    }
    fn on_exit(&mut self) {
        if let Err(e) = self.0.save() {
            eprintln!("Не удалось сохранить прогресс: {e}");
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = std::env::var_os("GRIDCRAFT_LEARN_STATE").map(std::path::PathBuf::from).unwrap_or_else(|| {
        let dir = std::env::var_os("XDG_DATA_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|p| std::path::PathBuf::from(p).join(".local/share")))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        dir.join("gridcraft-learn/progress.json")
    });
    if std::env::args().any(|a| a == "--remind") {
        // Called by the daily timer: one notification, no window.
        return gridcraft_learn::remind::run(&base, gridcraft_learn::now()).map_err(Into::into);
    }
    let mut app = LearningApp::new(Some(base))?;
    app.sheet.services = native::services();
    eframe::run_native(
        "Taskathlon",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 820.0]),
            renderer: eframe::Renderer::Wgpu,
            ..Default::default()
        },
        Box::new(move |cc| {
            gridcraft_ui_egui::SheetApp::setup_context_for_language(&cc.egui_ctx, false, gridcraft_ui_egui::i18n::Language::Ru);
            Ok(Box::new(Host(app)))
        }),
    )?;
    Ok(())
}
