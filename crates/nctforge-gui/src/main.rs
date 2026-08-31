// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::array;
use std::path::{Path, PathBuf};

use eframe::egui;
use nctforge_dicom::{VerifiedBenchmarkCase, load_nf_bnct_001};
use nctforge_openmc::OpenMcBackend;
use nctforge_transport::TransportBackend;
use nctforge_view::{AnatomicalPlane, Crosshair, PatientAlignedGrid, SliceView};

fn main() -> eframe::Result {
    let initial_case = std::env::args_os().nth(1).map(PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1_300.0, 820.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "NCTForge",
        options,
        Box::new(move |_creation_context| Ok(Box::new(NctForgeApp::new(initial_case)))),
    )
}

struct NctForgeApp {
    case_path: String,
    load_error: Option<String>,
    case: Option<ViewerCase>,
    display: DisplaySettings,
}

impl NctForgeApp {
    fn new(initial_case: Option<PathBuf>) -> Self {
        let mut app = Self {
            case_path: initial_case
                .as_deref()
                .map_or_else(String::new, |path| path.display().to_string()),
            load_error: None,
            case: None,
            display: DisplaySettings::default(),
        };
        if initial_case.is_some() {
            app.load_case();
        }
        app
    }

    fn load_case(&mut self) {
        let path = PathBuf::from(self.case_path.trim());
        if self.case_path.trim().is_empty() {
            self.load_error = Some("Enter an NF-BNCT-001 directory.".into());
            return;
        }
        match ViewerCase::load(&path) {
            Ok(case) => {
                self.case = Some(case);
                self.load_error = None;
            }
            Err(error) => {
                self.case = None;
                self.load_error = Some(error);
            }
        }
    }
}

impl eframe::App for NctForgeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.horizontal_wrapped(|ui| {
            ui.strong("RESEARCH SOFTWARE");
            ui.label("Not commissioned or certified for clinical decision-making.");
            ui.separator();
            ui.label("R1 geometry viewer — synthetic NF-BNCT-001 only");
        });
        ui.separator();

        let enter_pressed = ui.input(|input| input.key_pressed(egui::Key::Enter));
        ui.horizontal(|ui| {
            ui.label("Case directory");
            let path_response = ui.add(
                egui::TextEdit::singleline(&mut self.case_path)
                    .desired_width(520.0)
                    .hint_text("/tmp/nf-bnct-001"),
            );
            if ui.button("Load and verify").clicked()
                || (path_response.has_focus() && enter_pressed)
            {
                self.load_case();
            }
        });
        if let Some(error) = &self.load_error {
            ui.colored_label(egui::Color32::LIGHT_RED, format!("Load rejected: {error}"));
        }
        ui.separator();

        if let Some(case) = &mut self.case {
            show_loaded_case(ui, case, &mut self.display);
        } else {
            show_empty_state(ui);
        }
    }
}

struct DisplaySettings {
    window_center: f64,
    window_width: f64,
    overlay_opacity: f32,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            window_center: 0.0,
            window_width: 400.0,
            overlay_opacity: 0.48,
        }
    }
}

struct ViewerCase {
    root: PathBuf,
    verified: VerifiedBenchmarkCase,
    grid: PatientAlignedGrid,
    crosshair: Crosshair,
    roi_visible: Vec<bool>,
    textures: [Option<egui::TextureHandle>; 3],
    textures_dirty: bool,
}

impl ViewerCase {
    fn load(root: &Path) -> Result<Self, String> {
        let verified = load_nf_bnct_001(root).map_err(|error| error.to_string())?;
        let grid = PatientAlignedGrid::new(&verified.ct.geometry)
            .map_err(|error| format!("anatomical viewer cannot represent this grid: {error}"))?;
        let crosshair = Crosshair::centered(&grid);
        let roi_visible = verified
            .structures
            .rois
            .iter()
            .map(|roi| roi.name != "PHANTOM")
            .collect();
        Ok(Self {
            root: root.to_path_buf(),
            verified,
            grid,
            crosshair,
            roi_visible,
            textures: array::from_fn(|_| None),
            textures_dirty: true,
        })
    }

    fn refresh_textures(
        &mut self,
        context: &egui::Context,
        display: &DisplaySettings,
    ) -> Result<(), String> {
        if !self.textures_dirty {
            return Ok(());
        }
        for (view_index, plane) in AnatomicalPlane::ALL.into_iter().enumerate() {
            let view = self
                .grid
                .slice(plane, self.crosshair)
                .map_err(|error| error.to_string())?;
            let image = render_slice(&self.verified, view, &self.roi_visible, display)?;
            if let Some(texture) = &mut self.textures[view_index] {
                texture.set(image, egui::TextureOptions::NEAREST);
            } else {
                self.textures[view_index] = Some(context.load_texture(
                    format!("nctforge-{}-{}", self.root.display(), plane.name()),
                    image,
                    egui::TextureOptions::NEAREST,
                ));
            }
        }
        self.textures_dirty = false;
        Ok(())
    }
}

fn show_loaded_case(ui: &mut egui::Ui, case: &mut ViewerCase, display: &mut DisplaySettings) {
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_min_width(245.0);
            ui.set_max_width(280.0);
            show_case_summary(ui, case);
            ui.separator();
            if show_display_controls(ui, case, display) {
                case.textures_dirty = true;
            }
        });
        ui.separator();
        ui.vertical(|ui| {
            ui.heading("Linked anatomical views");
            ui.label("Click or drag in any view to move the shared voxel crosshair.");
            if let Err(error) = case.refresh_textures(ui.ctx(), display) {
                ui.colored_label(
                    egui::Color32::LIGHT_RED,
                    format!("Render rejected: {error}"),
                );
                return;
            }

            let mut selected_voxel = None;
            ui.columns(3, |columns| {
                for ((column, plane), texture) in columns
                    .iter_mut()
                    .zip(AnatomicalPlane::ALL)
                    .zip(case.textures.iter())
                {
                    if let Some(texture) = texture
                        && let Ok(view) = case.grid.slice(plane, case.crosshair)
                        && let Some(voxel) = show_slice_view(column, texture, view, case.crosshair)
                    {
                        selected_voxel = Some(voxel);
                    }
                }
            });
            if let Some(voxel) = selected_voxel
                && case.crosshair.set_voxel(&case.grid, voxel).is_ok()
            {
                case.textures_dirty = true;
            }
        });
    });
}

fn show_case_summary(ui: &mut egui::Ui, case: &ViewerCase) {
    ui.heading("Case");
    ui.strong(case.verified.report.case_id);
    ui.small(case.root.display().to_string());
    ui.label(format!(
        "Grid: {:?} at {:?} mm",
        case.verified.report.shape, case.verified.report.spacing_mm
    ));
    ui.colored_label(
        egui::Color32::LIGHT_GREEN,
        format!(
            "Integrity verified: {} DICOM files",
            case.verified.report.verified_artifact_count
        ),
    );
    ui.label("Qualification: synthetic research only");

    ui.separator();
    ui.heading("Crosshair");
    let voxel = case.crosshair.voxel();
    let world = case
        .crosshair
        .world_lps_mm(&case.grid)
        .unwrap_or([f64::NAN; 3]);
    ui.monospace(format!("voxel [{}, {}, {}]", voxel[0], voxel[1], voxel[2]));
    ui.monospace(format!(
        "LPS [{:.1}, {:.1}, {:.1}] mm",
        world[0], world[1], world[2]
    ));
    if let Ok(index) = case.grid.linear_index(voxel) {
        let stored = case.verified.ct.stored_pixels[index];
        ui.monospace(format!(
            "CT {:.1} HU (stored {stored})",
            case.verified.ct.modality_value(stored)
        ));
        let names: Vec<_> = case
            .verified
            .structures
            .rois
            .iter()
            .filter(|roi| roi.voxels[index])
            .map(|roi| roi.name.as_str())
            .collect();
        ui.label(if names.is_empty() {
            "ROIs: none".into()
        } else {
            format!("ROIs: {}", names.join(", "))
        });
    }

    ui.separator();
    ui.heading("Transport backends");
    let openmc = OpenMcBackend::default().descriptor();
    ui.label(format!("{} — adapter scaffold", openmc.display_name));
    ui.small("prepare=false, execute=false, import=false");
}

fn show_display_controls(
    ui: &mut egui::Ui,
    case: &mut ViewerCase,
    display: &mut DisplaySettings,
) -> bool {
    let mut changed = false;
    ui.heading("Display");
    changed |= ui
        .add(egui::Slider::new(&mut display.window_center, -1_024.0..=3_071.0).text("level HU"))
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut display.window_width, 1.0..=4_096.0).text("width HU"))
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut display.overlay_opacity, 0.0..=1.0).text("ROI opacity"))
        .changed();

    ui.separator();
    ui.heading("Linked voxel");
    let mut voxel = case.crosshair.voxel();
    for (axis, label) in ["column / L", "row / P", "slice / S"]
        .into_iter()
        .enumerate()
    {
        changed |= ui
            .add(
                egui::Slider::new(&mut voxel[axis], 0..=case.grid.geometry().shape[axis] - 1)
                    .text(label),
            )
            .changed();
    }
    if voxel != case.crosshair.voxel() {
        let _ = case.crosshair.set_voxel(&case.grid, voxel);
    }

    ui.separator();
    ui.heading("RT structures");
    for ((visible, roi), color) in case
        .roi_visible
        .iter_mut()
        .zip(&case.verified.structures.rois)
        .zip(
            case.verified
                .structures
                .rois
                .iter()
                .map(|roi| roi_color(roi.number)),
        )
    {
        ui.horizontal(|ui| {
            ui.colored_label(color, "■");
            changed |= ui.checkbox(visible, &roi.name).changed();
        });
    }
    changed
}

fn show_slice_view(
    ui: &mut egui::Ui,
    texture: &egui::TextureHandle,
    view: SliceView,
    crosshair: Crosshair,
) -> Option<[u32; 3]> {
    ui.strong(format!(
        "{} — index {}",
        view.plane().name(),
        view.fixed_index()
    ));
    let dimensions = view.dimensions();
    let spacing = view.pixel_spacing_mm();
    let physical_aspect = dimensions[0] as f64 * spacing[0] / (dimensions[1] as f64 * spacing[1]);
    let max_width = ui.available_width().max(120.0);
    let max_height = 420.0_f32;
    let image_size = if physical_aspect >= 1.0 {
        egui::vec2(
            max_width,
            (max_width / physical_aspect as f32).min(max_height),
        )
    } else {
        egui::vec2(
            (max_height * physical_aspect as f32).min(max_width),
            max_height,
        )
    };
    let response = ui.add(
        egui::Image::from_texture(texture)
            .fit_to_exact_size(image_size)
            .sense(egui::Sense::click_and_drag()),
    );
    paint_orientation_and_crosshair(ui, response.rect, view, crosshair);

    if (response.clicked() || response.dragged())
        && let Some(position) = response.interact_pointer_pos()
    {
        let fraction = [
            (position.x - response.rect.left()) / response.rect.width(),
            (position.y - response.rect.top()) / response.rect.height(),
        ];
        return view.voxel_at_fraction(fraction).ok();
    }
    None
}

fn paint_orientation_and_crosshair(
    ui: &egui::Ui,
    rect: egui::Rect,
    view: SliceView,
    crosshair: Crosshair,
) {
    let painter = ui.painter();
    let labels = view.edge_labels();
    let font = egui::FontId::proportional(15.0);
    let label_color = egui::Color32::WHITE;
    painter.text(
        rect.left_center() + egui::vec2(5.0, 0.0),
        egui::Align2::LEFT_CENTER,
        labels.left,
        font.clone(),
        label_color,
    );
    painter.text(
        rect.right_center() - egui::vec2(5.0, 0.0),
        egui::Align2::RIGHT_CENTER,
        labels.right,
        font.clone(),
        label_color,
    );
    painter.text(
        rect.center_top() + egui::vec2(0.0, 5.0),
        egui::Align2::CENTER_TOP,
        labels.top,
        font.clone(),
        label_color,
    );
    painter.text(
        rect.center_bottom() - egui::vec2(0.0, 5.0),
        egui::Align2::CENTER_BOTTOM,
        labels.bottom,
        font,
        label_color,
    );

    if let Ok(pixel) = view.pixel_for_voxel(crosshair.voxel()) {
        let dimensions = view.dimensions();
        let x = rect.left() + (pixel[0] as f32 + 0.5) / dimensions[0] as f32 * rect.width();
        let y = rect.top() + (pixel[1] as f32 + 0.5) / dimensions[1] as f32 * rect.height();
        let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 255, 255));
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            stroke,
        );
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            stroke,
        );
    }
}

fn render_slice(
    case: &VerifiedBenchmarkCase,
    view: SliceView,
    roi_visible: &[bool],
    display: &DisplaySettings,
) -> Result<egui::ColorImage, String> {
    let dimensions = view.dimensions();
    let mut pixels = Vec::with_capacity(dimensions[0] as usize * dimensions[1] as usize);
    for vertical in 0..dimensions[1] {
        for horizontal in 0..dimensions[0] {
            let index = view
                .linear_index_at([horizontal, vertical])
                .map_err(|error| error.to_string())?;
            let stored = case.ct.stored_pixels[index];
            let gray = window_to_gray(
                case.ct.modality_value(stored),
                display.window_center,
                display.window_width,
            );
            let mut color = egui::Color32::from_gray(gray);
            for ((visible, roi), overlay) in roi_visible
                .iter()
                .zip(&case.structures.rois)
                .zip(case.structures.rois.iter().map(|roi| roi_color(roi.number)))
            {
                if *visible && roi.voxels[index] {
                    color = blend(color, overlay, display.overlay_opacity);
                }
            }
            pixels.push(color);
        }
    }
    Ok(egui::ColorImage::new(
        [dimensions[0] as usize, dimensions[1] as usize],
        pixels,
    ))
}

fn window_to_gray(value: f64, center: f64, width: f64) -> u8 {
    let width = width.max(1.0);
    if width <= 1.0 {
        return if value <= center - 0.5 { 0 } else { 255 };
    }
    let lower = center - 0.5 - (width - 1.0) / 2.0;
    let upper = center - 0.5 + (width - 1.0) / 2.0;
    if value <= lower {
        0
    } else if value > upper {
        255
    } else {
        (((value - (center - 0.5)) / (width - 1.0) + 0.5) * 255.0).round() as u8
    }
}

fn blend(base: egui::Color32, overlay: egui::Color32, opacity: f32) -> egui::Color32 {
    let opacity = opacity.clamp(0.0, 1.0);
    let channel = |base: u8, overlay: u8| {
        (f32::from(base) * (1.0 - opacity) + f32::from(overlay) * opacity).round() as u8
    };
    egui::Color32::from_rgb(
        channel(base.r(), overlay.r()),
        channel(base.g(), overlay.g()),
        channel(base.b(), overlay.b()),
    )
}

fn roi_color(number: i32) -> egui::Color32 {
    match number {
        1 => egui::Color32::from_rgb(180, 180, 180),
        2 => egui::Color32::from_rgb(255, 80, 80),
        3 => egui::Color32::from_rgb(80, 255, 80),
        4 => egui::Color32::from_rgb(80, 80, 255),
        5 => egui::Color32::from_rgb(255, 255, 0),
        _ => egui::Color32::from_rgb(255, 0, 255),
    }
}

fn show_empty_state(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(90.0);
        ui.heading("No verified synthetic case loaded");
        ui.label("Generate the frozen case from a terminal:");
        ui.monospace("cargo run --bin nctforge -- benchmark generate /tmp/nf-bnct-001");
        ui.label("Then load that directory above, or pass it as the first GUI argument.");
        ui.add_space(20.0);
        ui.label("DICOM and artifact-integrity gates run before any image is displayed.");
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voi_window_has_expected_endpoints_and_midpoint() {
        assert_eq!(window_to_gray(-200.0, 0.0, 400.0), 0);
        assert_eq!(window_to_gray(199.0, 0.0, 400.0), 255);
        assert_eq!(window_to_gray(-0.5, 0.0, 400.0), 128);
    }

    #[test]
    fn overlay_blending_respects_zero_and_full_opacity() {
        let base = egui::Color32::from_rgb(10, 20, 30);
        let overlay = egui::Color32::from_rgb(210, 120, 60);
        assert_eq!(blend(base, overlay, 0.0), base);
        assert_eq!(blend(base, overlay, 1.0), overlay);
    }
}
