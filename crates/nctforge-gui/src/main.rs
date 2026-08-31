// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use eframe::egui;
use nctforge_openmc::OpenMcBackend;
use nctforge_transport::TransportBackend;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "NCTForge",
        options,
        Box::new(|_creation_context| Ok(Box::<NctForgeApp>::default())),
    )
}

#[derive(Default)]
struct NctForgeApp;

impl eframe::App for NctForgeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let openmc = OpenMcBackend::default().descriptor();

        ui.horizontal_wrapped(|ui| {
            ui.strong("RESEARCH SOFTWARE");
            ui.label("Not commissioned or certified for clinical decision-making.");
        });
        ui.separator();

        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_min_width(220.0);
                ui.heading("Case");
                ui.label("No DICOM case loaded");
                ui.separator();
                ui.heading("Transport backends");
                ui.label(format!("{} — adapter scaffold", openmc.display_name));
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.heading("NCTForge");
                ui.label("Transport-neutral BNCT research and verification workbench");
                ui.add_space(16.0);
                ui.columns(3, |columns| {
                    for (column, label) in columns.iter_mut().zip([
                        "Axial viewer",
                        "Sagittal viewer",
                        "Coronal viewer",
                    ]) {
                        column.group(|ui| {
                            ui.set_min_height(240.0);
                            ui.centered_and_justified(|ui| ui.label(label));
                        });
                    }
                });
                ui.add_space(8.0);
                ui.label(
                    "The first milestone will replace these panes with linked synthetic CT slices and component-dose overlays.",
                );
            });
        });
    }
}
