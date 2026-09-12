// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

mod brand;
mod help;

use std::array;
use std::path::{Path, PathBuf};

use eframe::egui;
use nctforge_bio::{BiologicalDoseBundle, RegionMask};
use nctforge_core::PhysicalDoseBundle;
use nctforge_dicom::{VerifiedBenchmarkCase, load_nf_bnct_001};
use nctforge_evidence::{DoseVolumeHistogram, EvidenceBundleManifest, sha256_hex};
use nctforge_openmc::{OpenMcBackend, TARGET_OPENMC_VERSION};
use nctforge_transport::TransportBackend;
use nctforge_view::{AnatomicalPlane, Crosshair, PatientAlignedGrid, SliceView};

use help::{GuidedHelp, HelpWorkspace, TourTarget, TourTargets};

const OPENMC_MANIFEST_EVIDENCE: &[u8] = include_bytes!(
    "../../../benchmarks/synthetic/nf-bnct-001/transport/provenance/openmc-endfb81-processed-data-manifest.json"
);
const NJOY_EXECUTION_EVIDENCE: &[u8] = include_bytes!(
    "../../../benchmarks/synthetic/nf-bnct-001/transport/provenance/njoy2016-78-execution-receipt.json"
);
const HEATING_COMPARISON_EVIDENCE: &[u8] = include_bytes!(
    "../../../benchmarks/synthetic/nf-bnct-001/transport/provenance/openmc-njoy-mt301-comparison.json"
);

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
        Box::new(move |creation_context| {
            configure_style(&creation_context.egui_ctx);
            Ok(Box::new(NctForgeApp::new(
                initial_case,
                &creation_context.egui_ctx,
            )))
        }),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum WorkspaceTab {
    #[default]
    Overview,
    Geometry,
    Transport,
    Dose,
    Evidence,
}

impl WorkspaceTab {
    const ALL: [Self; 5] = [
        Self::Overview,
        Self::Geometry,
        Self::Transport,
        Self::Dose,
        Self::Evidence,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Geometry => "Geometry",
            Self::Transport => "Transport",
            Self::Dose => "Dose components",
            Self::Evidence => "Evidence",
        }
    }

    const fn marker(self) -> &'static str {
        match self {
            Self::Overview => "01",
            Self::Geometry => "02",
            Self::Transport => "03",
            Self::Dose => "04",
            Self::Evidence => "05",
        }
    }
}

impl From<WorkspaceTab> for HelpWorkspace {
    fn from(workspace: WorkspaceTab) -> Self {
        match workspace {
            WorkspaceTab::Overview => Self::Overview,
            WorkspaceTab::Geometry => Self::Geometry,
            WorkspaceTab::Transport => Self::Transport,
            WorkspaceTab::Dose => Self::Dose,
            WorkspaceTab::Evidence => Self::Evidence,
        }
    }
}

impl From<HelpWorkspace> for WorkspaceTab {
    fn from(workspace: HelpWorkspace) -> Self {
        match workspace {
            HelpWorkspace::Overview => Self::Overview,
            HelpWorkspace::Geometry => Self::Geometry,
            HelpWorkspace::Transport => Self::Transport,
            HelpWorkspace::Dose => Self::Dose,
            HelpWorkspace::Evidence => Self::Evidence,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GateState {
    Verified,
    Frozen,
    Blocked,
    Pending,
    InputRequired,
}

impl GateState {
    const fn label(self) -> &'static str {
        match self {
            Self::Verified => "VERIFIED",
            Self::Frozen => "FROZEN",
            Self::Blocked => "BLOCKED",
            Self::Pending => "PENDING",
            Self::InputRequired => "INPUT REQUIRED",
        }
    }

    fn color(self) -> egui::Color32 {
        match self {
            Self::Verified => egui::Color32::from_rgb(90, 210, 153),
            Self::Frozen => egui::Color32::from_rgb(91, 166, 255),
            Self::Blocked => egui::Color32::from_rgb(244, 171, 67),
            Self::Pending => egui::Color32::from_rgb(151, 158, 178),
            Self::InputRequired => egui::Color32::from_rgb(214, 117, 117),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReadinessGate {
    title: &'static str,
    detail: &'static str,
    state: GateState,
}

fn readiness_gates(case_loaded: bool) -> [ReadinessGate; 5] {
    [
        ReadinessGate {
            title: "DICOM geometry",
            detail: if case_loaded {
                "Case artifacts and patient-space geometry passed the runtime gate."
            } else {
                "Load NF-BNCT-001 to run the DICOM and integrity gate."
            },
            state: if case_loaded {
                GateState::Verified
            } else {
                GateState::InputRequired
            },
        },
        ReadinessGate {
            title: "Material and source",
            detail: "Versioned NF-BNCT-001 benchmark contracts are checked in.",
            state: GateState::Frozen,
        },
        ReadinessGate {
            title: "OpenMC nuclear data",
            detail: "Official case selection and 16 artifact identities are frozen.",
            state: GateState::Frozen,
        },
        ReadinessGate {
            title: "Component responses",
            detail: "O-17/O-18 transported-photon treatment requires independent review.",
            state: GateState::Blocked,
        },
        ReadinessGate {
            title: "Controlled transport run",
            detail: "Disabled until every upstream scientific gate passes.",
            state: GateState::Pending,
        },
    ]
}

/// A loaded dose artifact — physical or biological — with the same contract
/// validation the CLI enforces. Weighted bundles stay visually distinct.
enum DoseArtifact {
    Physical(PhysicalDoseBundle),
    Biological(BiologicalDoseBundle),
}

struct LoadedDose {
    artifact: DoseArtifact,
    sha256: String,
}

/// (component label, values, sigma) rows for display.
type DoseRows<'a> = Vec<(String, &'a [f64], Option<&'a [f64]>)>;

impl DoseArtifact {
    fn load(path: &Path) -> Result<LoadedDose, String> {
        let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let sha256 = sha256_hex(&bytes);
        let schema: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        let artifact = match schema
            .get("schema_version")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
        {
            nctforge_core::PHYSICAL_DOSE_BUNDLE_SCHEMA => {
                let bundle: PhysicalDoseBundle =
                    serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
                bundle.validate().map_err(|error| error.to_string())?;
                Self::Physical(bundle)
            }
            nctforge_bio::BIOLOGICAL_DOSE_BUNDLE_SCHEMA => {
                let bundle: BiologicalDoseBundle =
                    serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
                bundle.validate().map_err(|error| error.to_string())?;
                Self::Biological(bundle)
            }
            other => return Err(format!("unsupported dose bundle schema {other:?}")),
        };
        Ok(LoadedDose { artifact, sha256 })
    }

    fn case_id(&self) -> &str {
        match self {
            Self::Physical(bundle) => &bundle.case_id,
            Self::Biological(bundle) => &bundle.case_id,
        }
    }

    fn unit(&self) -> &str {
        match self {
            Self::Physical(bundle) => match bundle.components.first() {
                Some(volume) => match volume.unit {
                    nctforge_core::DoseUnit::Gray => "gray",
                    nctforge_core::DoseUnit::GrayPerSourceParticle => "gray_per_source_particle",
                },
                None => "unknown",
            },
            Self::Biological(bundle) => &bundle.unit,
        }
    }

    fn rows(&self) -> DoseRows<'_> {
        match self {
            Self::Physical(bundle) => {
                let mut rows: DoseRows<'_> = bundle
                    .components
                    .iter()
                    .map(|volume| {
                        (
                            serde_json::to_value(volume.component)
                                .and_then(|v| {
                                    v.as_str().map(str::to_owned).ok_or(serde_json::Error::io(
                                        std::io::Error::other("component not a string"),
                                    ))
                                })
                                .unwrap_or_else(|_| "unknown".into()),
                            volume.values.as_slice(),
                            volume.absolute_standard_uncertainty.as_deref(),
                        )
                    })
                    .collect();
                rows.push((
                    "physical_total".into(),
                    bundle.physical_total.values.as_slice(),
                    bundle
                        .physical_total
                        .absolute_standard_uncertainty
                        .as_deref(),
                ));
                rows
            }
            Self::Biological(bundle) => {
                let mut rows: DoseRows<'_> = bundle
                    .components
                    .iter()
                    .map(|volume| {
                        (
                            serde_json::to_value(volume.component)
                                .and_then(|v| {
                                    v.as_str().map(str::to_owned).ok_or(serde_json::Error::io(
                                        std::io::Error::other("component not a string"),
                                    ))
                                })
                                .unwrap_or_else(|_| "unknown".into()),
                            volume.values.as_slice(),
                            volume.absolute_standard_uncertainty.as_deref(),
                        )
                    })
                    .collect();
                rows.push((
                    "biological_total".into(),
                    bundle.total.values.as_slice(),
                    bundle.total.absolute_standard_uncertainty.as_deref(),
                ));
                rows
            }
        }
    }

    fn qualification(&self) -> &'static str {
        match self {
            Self::Physical(_) => "synthetic_research_only",
            Self::Biological(bundle) => match bundle.qualification.as_str() {
                "synthetic_research_only_not_clinical" => "synthetic_research_only_not_clinical",
                _ => "unqualified",
            },
        }
    }
}

/// UI state for the dose workspace: the loaded bundle plus the region mask
/// and quantity chosen for the DVH panel.
#[derive(Default)]
struct DosePanel {
    bundle_path: String,
    bundle: Option<LoadedDose>,
    bundle_error: Option<String>,
    mask_path: String,
    quantity: String,
    histogram: Option<DoseVolumeHistogram>,
    histogram_error: Option<String>,
}

impl DosePanel {
    fn load_bundle(&mut self) {
        match DoseArtifact::load(Path::new(self.bundle_path.trim())) {
            Ok(bundle) => {
                self.bundle_error = None;
                self.histogram = None;
                self.quantity = match &bundle.artifact {
                    DoseArtifact::Physical(_) => "physical_total".into(),
                    DoseArtifact::Biological(_) => "biological_total".into(),
                };
                self.bundle = Some(bundle);
            }
            Err(error) => {
                self.bundle = None;
                self.bundle_error = Some(error);
            }
        }
    }

    fn compute_histogram(&mut self) {
        self.histogram = None;
        self.histogram_error = None;
        let Some(bundle) = &self.bundle else {
            self.histogram_error = Some("Load a dose bundle first.".into());
            return;
        };
        let mask: RegionMask = match std::fs::read(self.mask_path.trim())
            .map_err(|e| e.to_string())
            .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|e| e.to_string()))
        {
            Ok(mask) => mask,
            Err(error) => {
                self.histogram_error = Some(format!("mask: {error}"));
                return;
            }
        };
        let quantity = self.quantity.trim().to_owned();
        let resolved = bundle
            .artifact
            .rows()
            .into_iter()
            .find(|(name, ..)| *name == quantity || format!("component:{name}") == quantity)
            .map(|(_, values, _)| values);
        let Some(values) = resolved else {
            self.histogram_error = Some(format!("unknown quantity {quantity:?}"));
            return;
        };
        match DoseVolumeHistogram::compute(
            bundle.artifact.case_id(),
            &mask.name,
            &quantity,
            nctforge_core::ContentReference {
                id: bundle.artifact.case_id().to_owned(),
                sha256: bundle.sha256.clone(),
            },
            bundle.artifact.unit(),
            values,
            &mask.voxels,
            match &bundle.artifact {
                DoseArtifact::Physical(b) => b.geometry.spacing_mm.iter().product(),
                DoseArtifact::Biological(b) => b.geometry.spacing_mm.iter().product(),
            },
            100,
        ) {
            Ok(histogram) => self.histogram = Some(histogram),
            Err(error) => self.histogram_error = Some(error.to_string()),
        }
    }
}

/// UI state for the evidence workspace's bundle-verification panel.
#[derive(Default)]
struct EvidencePanel {
    root: String,
    manifest: Option<EvidenceBundleManifest>,
    error: Option<String>,
}

impl EvidencePanel {
    fn verify(&mut self) {
        match EvidenceBundleManifest::load_verified(Path::new(self.root.trim())) {
            Ok(manifest) => {
                self.manifest = Some(manifest);
                self.error = None;
            }
            Err(error) => {
                self.manifest = None;
                self.error = Some(error.to_string());
            }
        }
    }
}

struct NctForgeApp {
    case_path: String,
    load_error: Option<String>,
    case: Option<ViewerCase>,
    display: DisplaySettings,
    workspace: WorkspaceTab,
    brand_logo: Option<egui::TextureHandle>,
    help: GuidedHelp,
    dose_panel: DosePanel,
    evidence_panel: EvidencePanel,
}

impl NctForgeApp {
    fn new(initial_case: Option<PathBuf>, context: &egui::Context) -> Self {
        let has_initial_case = initial_case.is_some();
        let mut app = Self {
            case_path: initial_case
                .as_deref()
                .map_or_else(String::new, |path| path.display().to_string()),
            load_error: None,
            case: None,
            display: DisplaySettings::default(),
            workspace: if has_initial_case {
                WorkspaceTab::Geometry
            } else {
                WorkspaceTab::Overview
            },
            brand_logo: brand::load_logo_texture(context).ok(),
            help: GuidedHelp::default(),
            dose_panel: DosePanel::default(),
            evidence_panel: EvidencePanel::default(),
        };
        if has_initial_case {
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
        if ui.input(|input| input.key_pressed(egui::Key::F1)) {
            self.help.toggle_center();
        }
        if let Some(workspace) = self.help.requested_workspace() {
            self.workspace = workspace.into();
        }

        let mut tour_targets = TourTargets::default();
        if show_app_header(
            ui,
            self.case.as_ref(),
            self.brand_logo.as_ref(),
            &mut tour_targets,
        ) {
            self.help.toggle_center();
        }
        ui.add_space(8.0);
        let enter_pressed = ui.input(|input| input.key_pressed(egui::Key::Enter));
        let load_requested = show_case_loader(
            ui,
            &mut self.case_path,
            self.case.as_ref(),
            enter_pressed,
            &mut tour_targets,
        );
        if load_requested {
            self.load_case();
        }
        if let Some(error) = &self.load_error {
            ui.colored_label(egui::Color32::LIGHT_RED, format!("Load rejected: {error}"));
        }
        ui.add_space(8.0);
        ui.separator();
        show_workbench(
            ui,
            &mut self.workspace,
            self.case.as_mut(),
            &mut self.display,
            &mut self.dose_panel,
            &mut self.evidence_panel,
            &mut tour_targets,
        );
        self.help
            .show_center(ui.ctx(), self.workspace.into(), self.case.is_some());
        self.help.show_tour(ui.ctx(), &tour_targets);
    }
}

fn configure_style(context: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(17, 21, 29);
    visuals.window_fill = egui::Color32::from_rgb(21, 26, 36);
    visuals.extreme_bg_color = egui::Color32::from_rgb(10, 13, 19);
    visuals.faint_bg_color = egui::Color32::from_rgb(27, 33, 44);
    visuals.selection.bg_fill = egui::Color32::from_rgb(30, 116, 138);
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(144, 231, 239));
    context.set_visuals(visuals);

    context.global_style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
    });
}

fn show_app_header(
    ui: &mut egui::Ui,
    case: Option<&ViewerCase>,
    brand_logo: Option<&egui::TextureHandle>,
    tour_targets: &mut TourTargets,
) -> bool {
    let mut help_clicked = false;
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(21, 30, 40))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(brand_logo) = brand_logo {
                    let response = ui
                        .add(
                            egui::Image::from_texture(brand_logo)
                                .fit_to_exact_size(egui::vec2(48.0, 48.0))
                                .corner_radius(6.0),
                        )
                        .on_hover_text("Avila Labs");
                    tour_targets.set(TourTarget::Brand, response.rect);
                } else {
                    let response = ui.label(
                        egui::RichText::new("AVILA LABS")
                            .small()
                            .strong()
                            .color(egui::Color32::from_rgb(139, 229, 235)),
                    );
                    tour_targets.set(TourTarget::Brand, response.rect);
                }
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("NCTFORGE")
                            .size(24.0)
                            .strong()
                            .color(egui::Color32::from_rgb(139, 229, 235)),
                    );
                    ui.label(
                        egui::RichText::new("Open BNCT research and verification workbench")
                            .color(egui::Color32::from_rgb(183, 192, 209)),
                    );
                    ui.label(
                        egui::RichText::new("AN AVILA LABS OPEN-SOURCE PROJECT")
                            .size(9.5)
                            .strong()
                            .color(egui::Color32::from_rgb(137, 146, 165)),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let response = ui
                        .add_sized(
                            [36.0, 36.0],
                            egui::Button::new(egui::RichText::new("?").size(19.0).strong()),
                        )
                        .on_hover_text("Help, common questions, and guided tours (F1)");
                    help_clicked = response.clicked();
                    tour_targets.set(TourTarget::HelpButton, response.rect);
                    status_badge(ui, GateState::Pending, "RESEARCH ONLY");
                    if case.is_some() {
                        status_badge(ui, GateState::Verified, "CASE VERIFIED");
                    }
                });
            });
        });
    ui.horizontal_wrapped(|ui| {
        ui.colored_label(
            egui::Color32::from_rgb(244, 188, 95),
            egui::RichText::new("NOT FOR CLINICAL DECISION-MAKING").strong(),
        );
        ui.label("No dose, prescription, or treatment-delivery claim is available in this build.");
    });
    help_clicked
}

fn show_case_loader(
    ui: &mut egui::Ui,
    case_path: &mut String,
    case: Option<&ViewerCase>,
    enter_pressed: bool,
    tour_targets: &mut TourTargets,
) -> bool {
    let mut load_requested = false;
    let response = egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("CASE").small().strong());
            let path_response = ui.add(
                egui::TextEdit::singleline(case_path)
                    .desired_width(520.0)
                    .hint_text("/tmp/nf-bnct-001"),
            );
            load_requested = ui.button("Load + verify").clicked()
                || (path_response.has_focus() && enter_pressed);
            if let Some(case) = case {
                ui.separator();
                ui.strong(case.verified.report.case_id);
                ui.label(format!(
                    "{} artifacts",
                    case.verified.report.verified_artifact_count
                ));
            }
        });
    });
    tour_targets.set(TourTarget::CaseLoader, response.response.rect);
    load_requested
}

fn show_workbench(
    ui: &mut egui::Ui,
    workspace: &mut WorkspaceTab,
    case: Option<&mut ViewerCase>,
    display: &mut DisplaySettings,
    dose_panel: &mut DosePanel,
    evidence_panel: &mut EvidencePanel,
    tour_targets: &mut TourTargets,
) {
    let navigation = egui::Panel::left("nctforge-workspace-navigation")
        .exact_size(180.0)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(15, 19, 27))
                .inner_margin(egui::Margin::symmetric(10, 12)),
        )
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("WORKSPACES")
                    .small()
                    .strong()
                    .color(egui::Color32::from_rgb(137, 146, 165)),
            );
            for candidate in WorkspaceTab::ALL {
                let label = format!("{}  {}", candidate.marker(), candidate.label());
                if ui
                    .selectable_label(*workspace == candidate, label)
                    .clicked()
                {
                    *workspace = candidate;
                }
            }
            ui.add_space(16.0);
            ui.separator();
            ui.small("Milestone");
            ui.strong("R2 · physical truth");
            ui.small("Evidence-gated; no calendar-based completion claims.");
        });
    tour_targets.set(TourTarget::WorkspaceNavigation, navigation.response.rect);
    egui::CentralPanel::default()
        .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(12, 8)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("nctforge-workspace")
                .auto_shrink([false, false])
                .show(ui, |ui| match *workspace {
                    WorkspaceTab::Overview => show_overview(ui, case.as_deref(), tour_targets),
                    WorkspaceTab::Geometry => {
                        if let Some(case) = case {
                            show_geometry_workspace(ui, case, display, tour_targets);
                        } else {
                            show_workspace_heading(
                                ui,
                                "Geometry",
                                "Patient-space DICOM truth before transport.",
                            );
                            show_empty_state(ui);
                        }
                    }
                    WorkspaceTab::Transport => {
                        show_transport_workspace(ui, case.as_deref(), tour_targets)
                    }
                    WorkspaceTab::Dose => show_dose_workspace(ui, dose_panel),
                    WorkspaceTab::Evidence => {
                        show_evidence_workspace(ui, case.as_deref(), evidence_panel, tour_targets)
                    }
                });
        });
}

fn show_workspace_heading(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.heading(egui::RichText::new(title).size(23.0));
    ui.label(egui::RichText::new(subtitle).color(egui::Color32::from_rgb(164, 174, 193)));
    ui.add_space(8.0);
}

fn show_overview(ui: &mut egui::Ui, case: Option<&ViewerCase>, tour_targets: &mut TourTargets) {
    show_workspace_heading(
        ui,
        "Research overview",
        "One place to see what is verified, what is frozen, and what still blocks a result.",
    );

    egui::Frame::new()
        .fill(egui::Color32::from_rgb(19, 36, 45))
        .corner_radius(8)
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("NF-BNCT-001")
                            .size(20.0)
                            .strong()
                            .color(egui::Color32::from_rgb(139, 229, 235)),
                    );
                    ui.label("Synthetic conformance case · macroscopic physical dose");
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    status_badge(ui, GateState::Blocked, "RESPONSE GATE");
                    status_badge(
                        ui,
                        if case.is_some() {
                            GateState::Verified
                        } else {
                            GateState::InputRequired
                        },
                        if case.is_some() {
                            "GEOMETRY READY"
                        } else {
                            "LOAD GEOMETRY"
                        },
                    );
                });
            });
        });

    ui.add_space(12.0);
    let gates = ui.scope(|ui| {
        ui.heading("Readiness gates");
        ui.columns(2, |columns| {
            for (index, gate) in readiness_gates(case.is_some()).into_iter().enumerate() {
                show_gate_card(&mut columns[index % 2], gate);
            }
        });
    });
    tour_targets.set(TourTarget::OverviewGates, gates.response.rect);

    ui.add_space(12.0);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(egui::RichText::new("NEXT SCIENTIFIC DECISION").small().strong());
        ui.heading("Resolve O-17/O-18 transported-photon response semantics");
        ui.label(
            "The official OpenMC tables reproduce the controlled NJOY MT 301 curves. "
                .to_owned()
                + "That confirms the data gap; it does not authorize a zero or a hidden local-deposition fallback.",
        );
    });
}

fn show_gate_card(ui: &mut egui::Ui, gate: ReadinessGate) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_height(78.0);
        ui.horizontal(|ui| {
            ui.colored_label(gate.state.color(), "●");
            ui.vertical(|ui| {
                ui.strong(gate.title);
                ui.small(gate.detail);
                ui.label(
                    egui::RichText::new(gate.state.label())
                        .small()
                        .strong()
                        .color(gate.state.color()),
                );
            });
        });
    });
}

fn status_badge(ui: &mut egui::Ui, state: GateState, label: &str) {
    egui::Frame::new()
        .fill(state.color().gamma_multiply(0.14))
        .stroke(egui::Stroke::new(1.0, state.color().gamma_multiply(0.75)))
        .corner_radius(5)
        .inner_margin(egui::Margin::symmetric(7, 3))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(label)
                    .small()
                    .strong()
                    .color(state.color()),
            );
        });
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

fn show_geometry_workspace(
    ui: &mut egui::Ui,
    case: &mut ViewerCase,
    display: &mut DisplaySettings,
    tour_targets: &mut TourTargets,
) {
    show_workspace_heading(
        ui,
        "Geometry",
        "Integrity-gated, linked patient-space views of the frozen synthetic case.",
    );
    ui.horizontal_top(|ui| {
        let controls = ui.vertical(|ui| {
            ui.set_min_width(245.0);
            ui.set_max_width(280.0);
            show_case_summary(ui, case);
            ui.separator();
            if show_display_controls(ui, case, display) {
                case.textures_dirty = true;
            }
        });
        tour_targets.set(TourTarget::GeometryControls, controls.response.rect);
        ui.separator();
        let views = ui.vertical(|ui| {
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
        tour_targets.set(TourTarget::GeometryViews, views.response.rect);
    });
}

fn show_transport_workspace(
    ui: &mut egui::Ui,
    case: Option<&ViewerCase>,
    tour_targets: &mut TourTargets,
) {
    show_workspace_heading(
        ui,
        "Transport",
        "Backend-neutral preparation with explicit scientific and execution gates.",
    );
    let backend = OpenMcBackend::default().descriptor();

    egui::Frame::new()
        .fill(egui::Color32::from_rgb(22, 30, 41))
        .corner_radius(8)
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.heading(format!("{} adapter", backend.display_name));
                    ui.label(format!(
                        "Pinned target {} · executable boundary: {}",
                        TARGET_OPENMC_VERSION, backend.id
                    ));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    status_badge(ui, GateState::Pending, "NOT EXECUTABLE")
                });
            });
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                capability_label(ui, "Prepare", backend.can_prepare);
                capability_label(ui, "Execute", backend.can_execute);
                capability_label(ui, "Import", backend.can_import);
            });
        });

    ui.add_space(12.0);
    let gate_chain = ui.scope(|ui| {
        ui.heading("Run gate chain");
        for (index, gate) in readiness_gates(case.is_some()).into_iter().enumerate() {
            ui.horizontal(|ui| {
                ui.monospace(format!("{:02}", index + 1));
                ui.colored_label(gate.state.color(), "●");
                ui.strong(gate.title);
                ui.label("—");
                ui.label(gate.detail);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(gate.state.label())
                            .small()
                            .strong()
                            .color(gate.state.color()),
                    );
                });
            });
            if index + 1 != readiness_gates(case.is_some()).len() {
                ui.separator();
            }
        }
    });
    tour_targets.set(TourTarget::TransportGates, gate_chain.response.rect);

    ui.add_space(14.0);
    let actions = ui.horizontal(|ui| {
        ui.add_enabled(false, egui::Button::new("Prepare OpenMC run"))
            .on_disabled_hover_text("Blocked until the reviewed component responses pass.");
        ui.add_enabled(false, egui::Button::new("Execute transport"))
            .on_disabled_hover_text("The backend does not advertise controlled execution yet.");
        ui.label("Disabled controls reflect real adapter capabilities.");
    });
    tour_targets.set(TourTarget::TransportActions, actions.response.rect);
}

fn capability_label(ui: &mut egui::Ui, name: &str, enabled: bool) {
    let (state, value) = if enabled {
        (GateState::Verified, "available")
    } else {
        (GateState::Pending, "unavailable")
    };
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.strong(name);
            ui.colored_label(state.color(), value);
        });
    });
}

fn show_dose_workspace(ui: &mut egui::Ui, panel: &mut DosePanel) {
    show_workspace_heading(
        ui,
        "Dose components",
        "Physical and biological layers stay separate; only validated bundles render.",
    );

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("DOSE BUNDLE").small().strong());
            ui.add(
                egui::TextEdit::singleline(&mut panel.bundle_path)
                    .desired_width(520.0)
                    .hint_text("/path/to/dose-bundle.json"),
            );
            if ui.button("Load + validate").clicked() {
                panel.load_bundle();
            }
        });
        if let Some(error) = &panel.bundle_error {
            ui.colored_label(egui::Color32::LIGHT_RED, format!("Load rejected: {error}"));
        }
    });

    let Some(bundle) = &panel.bundle else {
        ui.add_space(12.0);
        egui::Frame::new()
            .fill(egui::Color32::from_rgb(41, 32, 22))
            .corner_radius(8)
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                status_badge(ui, GateState::Pending, "NO RESULT LOADED");
                ui.label(
                    "NCTForge does not render placeholder dose values. Load a validated \
                     physical or biological dose bundle to inspect components, totals, \
                     and DVHs.",
                );
            });
        return;
    };

    ui.add_space(10.0);
    let artifact = &bundle.artifact;
    let is_biological = matches!(artifact, DoseArtifact::Biological(_));
    egui::Frame::new()
        .fill(if is_biological {
            egui::Color32::from_rgb(36, 26, 46)
        } else {
            egui::Color32::from_rgb(22, 30, 41)
        })
        .corner_radius(8)
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.heading(artifact.case_id());
                    ui.monospace(format!("sha256:{}", bundle.sha256));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    status_badge(ui, GateState::Verified, "VALIDATED");
                    status_badge(
                        ui,
                        if is_biological {
                            GateState::Blocked
                        } else {
                            GateState::Frozen
                        },
                        artifact.qualification().to_uppercase().as_str(),
                    );
                });
            });
            if is_biological {
                ui.colored_label(
                    egui::Color32::from_rgb(206, 121, 226),
                    "Biologically weighted — never aliases physical dose. Not a clinical quantity.",
                );
            }
        });

    ui.add_space(10.0);
    let rows = artifact.rows();
    let columns = rows.len().clamp(1, 5);
    ui.columns(columns, |columns| {
        for (column, (name, values, sigma)) in columns.iter_mut().zip(rows.iter()) {
            let (symbol, color) = match name.as_str() {
                "boron" => ("D_B", egui::Color32::from_rgb(92, 207, 171)),
                "nitrogen" => ("D_N", egui::Color32::from_rgb(99, 165, 244)),
                "hydrogen" => ("D_H", egui::Color32::from_rgb(228, 167, 91)),
                "photon" => ("D_gamma", egui::Color32::from_rgb(206, 121, 226)),
                _ => ("Σ", egui::Color32::from_rgb(151, 158, 178)),
            };
            let max = values.iter().copied().fold(0.0_f64, f64::max);
            let mean = values.iter().sum::<f64>() / values.len().max(1) as f64;
            egui::Frame::group(column.style()).show(column, |ui| {
                ui.set_min_height(120.0);
                ui.colored_label(color, egui::RichText::new(symbol).size(18.0).strong());
                ui.strong(name.as_str());
                ui.monospace(format!("max  {max:.3e}"));
                ui.monospace(format!("mean {mean:.3e}"));
                ui.small(if sigma.is_some() {
                    "1σ uncertainty present"
                } else {
                    "no uncertainty carried"
                });
            });
        }
    });
    ui.monospace(format!("unit: {}", artifact.unit()));

    ui.add_space(14.0);
    ui.heading("Region dose-volume histogram");
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label("Mask");
            ui.add(
                egui::TextEdit::singleline(&mut panel.mask_path)
                    .desired_width(360.0)
                    .hint_text("/path/to/region-mask.json"),
            );
            ui.label("Quantity");
            ui.add(
                egui::TextEdit::singleline(&mut panel.quantity)
                    .desired_width(170.0)
                    .hint_text("physical_total"),
            );
            if ui.button("Compute DVH").clicked() {
                panel.compute_histogram();
            }
        });
        if let Some(error) = &panel.histogram_error {
            ui.colored_label(egui::Color32::LIGHT_RED, format!("DVH rejected: {error}"));
        }
        if let Some(histogram) = &panel.histogram {
            ui.label(format!(
                "region {} · {} voxels · {:.1} mm³ · {}",
                histogram.region,
                histogram.region_voxel_count,
                histogram.region_volume_mm3,
                histogram.unit
            ));
            show_dvh_curve(ui, histogram);
        }
    });
}

/// Draw the cumulative V(d) curve directly — no plotting dependency.
fn show_dvh_curve(ui: &mut egui::Ui, histogram: &DoseVolumeHistogram) {
    let (response, painter) = ui.allocate_painter(
        egui::vec2(ui.available_width(), 180.0),
        egui::Sense::hover(),
    );
    let rect = response.rect.shrink2(egui::vec2(46.0, 12.0));
    painter.rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(72, 82, 99)),
        egui::StrokeKind::Inside,
    );
    let max_dose = histogram.dose_edges.last().copied().unwrap_or(0.0);
    if max_dose <= 0.0 {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "zero dose in region",
            egui::FontId::monospace(12.0),
            egui::Color32::from_rgb(151, 158, 178),
        );
        return;
    }
    let points: Vec<egui::Pos2> = histogram
        .dose_edges
        .iter()
        .zip(&histogram.cumulative_volume_fraction)
        .map(|(dose, fraction)| {
            egui::pos2(
                rect.left() + (dose / max_dose) as f32 * rect.width(),
                rect.bottom() - (*fraction as f32) * rect.height(),
            )
        })
        .collect();
    painter.add(egui::Shape::line(
        points,
        egui::Stroke::new(2.0, egui::Color32::from_rgb(139, 229, 235)),
    ));
    painter.text(
        egui::pos2(rect.left() - 8.0, rect.top()),
        egui::Align2::RIGHT_CENTER,
        "100%",
        egui::FontId::monospace(10.0),
        egui::Color32::from_rgb(137, 146, 165),
    );
    painter.text(
        egui::pos2(rect.left() - 8.0, rect.bottom()),
        egui::Align2::RIGHT_CENTER,
        "0%",
        egui::FontId::monospace(10.0),
        egui::Color32::from_rgb(137, 146, 165),
    );
    painter.text(
        egui::pos2(rect.right(), rect.bottom() + 4.0),
        egui::Align2::RIGHT_TOP,
        format!("{:.3e} {}", max_dose, histogram.unit),
        egui::FontId::monospace(10.0),
        egui::Color32::from_rgb(137, 146, 165),
    );
}

fn show_evidence_workspace(
    ui: &mut egui::Ui,
    case: Option<&ViewerCase>,
    panel: &mut EvidencePanel,
    tour_targets: &mut TourTargets,
) {
    show_workspace_heading(
        ui,
        "Evidence",
        "Qualification is a chain of scoped claims, not one global green check.",
    );

    let geometry_detail = case.map_or_else(
        || "No runtime case has been verified in this session.".to_owned(),
        |case| {
            format!(
                "{} DICOM artifacts verified for {}.",
                case.verified.report.verified_artifact_count, case.verified.report.case_id
            )
        },
    );
    let manifest_hash = short_evidence_hash(OPENMC_MANIFEST_EVIDENCE);
    let execution_hash = short_evidence_hash(NJOY_EXECUTION_EVIDENCE);
    let comparison_hash = short_evidence_hash(HEATING_COMPARISON_EVIDENCE);
    let ledger = ui.scope(|ui| {
        show_evidence_row(
            ui,
            "Runtime geometry gate",
            if case.is_some() {
                GateState::Verified
            } else {
                GateState::InputRequired
            },
            &geometry_detail,
            None,
        );
        show_evidence_row(
            ui,
            "Official OpenMC processed selection",
            GateState::Frozen,
            "Case manifest binds cross_sections.xml, ten neutron tables, and five photon tables.",
            Some(&manifest_hash),
        );
        show_evidence_row(
            ui,
            "Controlled NJOY2016.78 execution",
            GateState::Blocked,
            "Preserved rejected evidence: 72 kinematic findings across four nuclides.",
            Some(&execution_hash),
        );
        show_evidence_row(
            ui,
            "OpenMC / NJOY MT 301 comparison",
            GateState::Frozen,
            "All ten curves agree within 4.9e-7; O-17/O-18 local fallback remains explicit.",
            Some(&comparison_hash),
        );
    });
    tour_targets.set(TourTarget::EvidenceLedger, ledger.response.rect);

    ui.add_space(12.0);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.heading("Qualification ceiling");
        ui.label(
            egui::RichText::new("synthetic_research_only")
                .monospace()
                .strong(),
        );
        ui.label(
            "Acquisition identity, transport capability, response suitability, execution, "
                .to_owned()
                + "cross-code comparison, and experimental validation remain separate claims.",
        );
    });

    ui.add_space(14.0);
    ui.heading("Exported evidence bundle");
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label("Bundle root");
            ui.add(
                egui::TextEdit::singleline(&mut panel.root)
                    .desired_width(460.0)
                    .hint_text("/path/to/evidence-bundle"),
            );
            if ui.button("Verify manifest + hashes").clicked() {
                panel.verify();
            }
        });
        if let Some(error) = &panel.error {
            ui.colored_label(
                egui::Color32::LIGHT_RED,
                format!("Verification rejected: {error}"),
            );
        }
        if let Some(manifest) = &panel.manifest {
            status_badge(ui, GateState::Verified, "ALL ARTIFACTS VERIFIED");
            let qualification = serde_json::to_value(manifest.qualification.clone())
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned))
                .unwrap_or_else(|| "unknown".into());
            ui.monospace(format!("case: {} · {}", manifest.case_id, qualification));
            for artifact in &manifest.artifacts {
                ui.horizontal(|ui| {
                    ui.monospace(format!("{:28}", artifact.role));
                    ui.label(artifact.path.clone());
                    ui.monospace(format!("sha256:{}…", &artifact.sha256[..12]));
                });
            }
        }
    });
}

fn short_evidence_hash(bytes: &[u8]) -> String {
    sha256_hex(bytes)[..12].to_owned()
}

fn show_evidence_row(
    ui: &mut egui::Ui,
    title: &str,
    state: GateState,
    detail: &str,
    hash: Option<&str>,
) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.colored_label(state.color(), "●");
            ui.vertical(|ui| {
                ui.strong(title);
                ui.label(detail);
                if let Some(hash) = hash {
                    ui.monospace(format!("sha256:{hash}…"));
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                status_badge(ui, state, state.label())
            });
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

    #[test]
    fn workspace_navigation_has_stable_unique_labels() {
        let labels = WorkspaceTab::ALL.map(WorkspaceTab::label);
        assert_eq!(labels.len(), 5);
        assert!(labels.iter().all(|label| !label.is_empty()));
        for (index, left) in labels.iter().enumerate() {
            assert!(!labels[index + 1..].contains(left));
        }
    }

    #[test]
    fn readiness_never_promotes_the_blocked_response_or_transport_run() {
        for case_loaded in [false, true] {
            let gates = readiness_gates(case_loaded);
            assert_eq!(gates[3].state, GateState::Blocked);
            assert_eq!(gates[4].state, GateState::Pending);
        }
        assert_eq!(readiness_gates(false)[0].state, GateState::InputRequired);
        assert_eq!(readiness_gates(true)[0].state, GateState::Verified);
    }

    #[test]
    fn displayed_evidence_hashes_are_derived_from_frozen_bytes() {
        assert_eq!(
            sha256_hex(OPENMC_MANIFEST_EVIDENCE),
            "3eaae09921172199c34f3fb236ae082ea5ace4567e0e04d2afcce357add73fb1"
        );
        assert_eq!(
            sha256_hex(NJOY_EXECUTION_EVIDENCE),
            "65a21b57507e76a68b77349e92390ae03ebb8c38f6ed6cee66197aa5ee4adea7"
        );
        assert_eq!(
            sha256_hex(HEATING_COMPARISON_EVIDENCE),
            "e9b1ffc5e70e3e489f23f9e185d12a5edeb7525161eb3b81470233d33f36f1e7"
        );
    }

    fn physical_bundle_json() -> serde_json::Value {
        let reference = |id: &str| serde_json::json!({"id": id, "sha256": "a".repeat(64)});
        let component = |name: &str, value: f64, sigma: f64| {
            serde_json::json!({
                "component": name,
                "unit": "gray_per_source_particle",
                "values": [value, value],
                "absolute_standard_uncertainty": [sigma, sigma],
            })
        };
        serde_json::json!({
            "schema_version": "nctforge.physical-dose-bundle/0.2.0",
            "case_id": "synthetic-case",
            "frame_of_reference_uid": null,
            "geometry": {
                "shape": [2, 1, 1],
                "spacing_mm": [5.0, 5.0, 5.0],
                "origin_mm": [-2.5, -2.5, -2.5],
                "direction": [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            },
            "component_profile": reference("profile"),
            "response_set": reference("response"),
            "components": [
                component("boron", 1.0e-12, 1.0e-14),
                component("nitrogen", 2.0e-13, 2.0e-15),
                component("hydrogen", 5.0e-14, 5.0e-16),
                component("photon", 3.0e-13, 3.0e-15),
            ],
            "physical_total": {
                "unit": "gray_per_source_particle",
                "values": [1.75e-12, 1.75e-12],
                "absolute_standard_uncertainty": [1.1e-14, 1.1e-14],
                "uncertainty_method": "dedicated_estimator",
            },
            "provenance_id": "test-provenance",
        })
    }

    #[test]
    fn dose_artifact_loads_validates_and_exposes_component_rows() {
        let scratch = tempfile::tempdir().unwrap();
        let path = scratch.path().join("bundle.json");
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&physical_bundle_json()).unwrap(),
        )
        .unwrap();
        let loaded = DoseArtifact::load(&path).unwrap();
        assert!(matches!(loaded.artifact, DoseArtifact::Physical(_)));
        assert_eq!(loaded.artifact.case_id(), "synthetic-case");
        assert_eq!(loaded.artifact.qualification(), "synthetic_research_only");
        let rows = loaded.artifact.rows();
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[4].0, "physical_total");
        assert_eq!(rows[4].1, &[1.75e-12, 1.75e-12]);
        assert_eq!(loaded.sha256.len(), 64);
    }

    #[test]
    fn dose_artifact_rejects_unknown_schema_and_invalid_bundles() {
        let scratch = tempfile::tempdir().unwrap();
        let bad_schema = scratch.path().join("bad.json");
        std::fs::write(&bad_schema, b"{\"schema_version\": \"other/9.9.9\"}").unwrap();
        assert!(DoseArtifact::load(&bad_schema).is_err());

        let mut invalid = physical_bundle_json();
        invalid["physical_total"]["values"] = serde_json::json!([1.0]);
        let invalid_path = scratch.path().join("invalid.json");
        std::fs::write(&invalid_path, serde_json::to_vec(&invalid).unwrap()).unwrap();
        assert!(DoseArtifact::load(&invalid_path).is_err());
    }

    #[test]
    fn every_empty_workspace_renders_at_the_minimum_viewport() {
        let context = egui::Context::default();
        configure_style(&context);
        for mut workspace in WorkspaceTab::ALL {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(960.0, 640.0),
                )),
                ..Default::default()
            };
            let mut display = DisplaySettings::default();
            let mut tour_targets = TourTargets::default();
            let mut output = context.run_ui(input, |ui| {
                show_workbench(
                    ui,
                    &mut workspace,
                    None,
                    &mut display,
                    &mut DosePanel::default(),
                    &mut EvidencePanel::default(),
                    &mut tour_targets,
                );
            });
            output.textures_delta.clear();
        }
    }
}
