pub mod log_file_reader;
pub mod log_view;
pub mod search_window;

use std::path::{Path, PathBuf};

use egui::{RichText, Ui};
use egui_dock::Tree;

use rfd::FileDialog;

use self::log_view::LogView;

struct LogViewTabViewer;

impl egui_dock::TabViewer for LogViewTabViewer {
    type Tab = LogView;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        tab.ui(ui);
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab.file_path().file_name() {
            Some(file_name) => file_name.to_string_lossy().into(),
            None => "Error".into(),
        }
    }
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    tree: Tree<LogView>,

    recent_files: Vec<PathBuf>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            tree: Tree::new(vec![]),
            recent_files: vec![],
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    fn add_recent_file(&mut self, logfile_path: &Path) {
        if !self.recent_files.iter().any(|f| f == logfile_path) {
            self.recent_files.push(logfile_path.to_path_buf())
        }
    }

    fn open_file(&mut self, file_path: Option<&Path>) -> Option<()> {
        let file_to_open = match file_path {
            Some(existing_path) => Some(existing_path.to_owned()),
            None => FileDialog::new()
                .add_filter("JSON Logs", &["log", "json"])
                .add_filter("Any", &["*"])
                .pick_file(),
        };

        if let Some(ref file_path) = file_to_open {
            self.tree.push_to_first_leaf(LogView::open(file_path).ok()?);
            self.add_recent_file(file_path);
        }

        Some(())
    }

    fn recent_file_menu(&self, ui: &mut Ui) -> Option<PathBuf> {
        for file in self
            .recent_files
            .iter()
            .map(|p| (p, p.file_name()))
            .filter(|s| s.1.is_some())
            .map(|s| (s.0, s.1.unwrap().to_string_lossy()))
        {
            if ui.button(file.1).clicked() {
                return Some(file.0.to_path_buf());
            }
        }
        None
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open...").clicked() {
                        self.open_file(None);
                        ui.close_menu();
                    }

                    ui.menu_button("Open Recent", |ui| {
                        if let Some(ref file_to_open) = self.recent_file_menu(ui) {
                            self.open_file(Some(&file_to_open));
                            ui.close_menu();
                        }
                    });

                    ui.separator();

                    if ui.button("Reset UI").clicked() {
                        ui.memory_mut(|mem| *mem = Default::default());
                        ui.close_menu();
                    }

                    if ui.button("Quit").clicked() {
                        frame.close();
                    }
                });

                if ui.button("Search").clicked() {
                    if let Some((_, log_view)) = self.tree.find_active() {
                        log_view.open_search();
                        ui.close_menu();
                    }
                }
            });
        });

        if !self.tree.is_empty() {
            egui_dock::DockArea::new(&mut self.tree).show(ctx, &mut LogViewTabViewer {})
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered_justified(|ui| {
                    ui.set_width(300.0);
                    ui.label(RichText::new("Welcome to JSON Log Viewer").strong());
                    if ui.button("Open File...").clicked() {
                        self.open_file(None);
                    }
                    ui.label("Recent Files");
                    ui.separator();

                    ui.style_mut().visuals.button_frame = false;
                    if let Some(selected_file) = self.recent_file_menu(ui) {
                        self.open_file(Some(&selected_file));
                    }
                });
            });
        }
    }
}
