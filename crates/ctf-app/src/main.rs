use ctf_core::{OperationInput, OperationRegistry, OperationRequest, TaskLimits};
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1180.0, 760.0]),
        ..Default::default()
    };

    eframe::run_native(
        "CTF Tools",
        options,
        Box::new(|cc| {
            install_cjk_font(&cc.egui_ctx);
            Ok(Box::new(CtfToolsApp::new()))
        }),
    )
}

fn install_cjk_font(ctx: &egui::Context) {
    let candidates = [
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
    ];
    let Some(bytes) = candidates.iter().find_map(|path| std::fs::read(path).ok()) else {
        return;
    };

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "system_cjk".to_owned(),
        egui::FontData::from_owned(bytes).into(),
    );

    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "system_cjk".to_owned());
    }

    ctx.set_fonts(fonts);
}

struct CtfToolsApp {
    registry: Option<OperationRegistry>,
    query: String,
    selected_operation: Option<String>,
    input: String,
    output: String,
    warnings: Vec<String>,
    last_status: String,
}

impl CtfToolsApp {
    fn new() -> Self {
        let registry = OperationRegistry::load_default().ok();
        let selected_operation = registry
            .as_ref()
            .and_then(|registry| registry.operations().first())
            .map(|op| op.id.clone());

        Self {
            registry,
            query: String::new(),
            selected_operation,
            input: "ZmxhZ3t0ZXN0fQ==".to_string(),
            output: String::new(),
            warnings: Vec::new(),
            last_status: "Ready".to_string(),
        }
    }

    fn run_selected(&mut self) {
        let (Some(registry), Some(operation)) =
            (self.registry.clone(), self.selected_operation.clone())
        else {
            self.output = "No operation selected".to_string();
            self.last_status = "Error".to_string();
            return;
        };

        let runner = ctf_runner::default_runner()
            .unwrap_or_else(|_| ctf_core::OperationRunner::new(registry));
        let result = runner.run(OperationRequest {
            operation,
            input: OperationInput {
                kind: "text".to_string(),
                value: self.input.clone(),
            },
            limits: TaskLimits::default(),
        });

        match result {
            Ok(response) => {
                self.output = response
                    .outputs
                    .iter()
                    .map(|output| output.value.clone())
                    .collect::<Vec<_>>()
                    .join("\n");
                self.warnings = response.warnings;
                self.last_status = "OK".to_string();
            }
            Err(error) => {
                self.output = error.to_string();
                self.warnings.clear();
                self.last_status = "Error".to_string();
            }
        }
    }
}

impl eframe::App for CtfToolsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let selected_spec = self.registry.as_ref().and_then(|registry| {
            self.selected_operation
                .as_deref()
                .and_then(|id| registry.find(id))
                .cloned()
        });

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("CTF Tools");
                ui.separator();
                ui.label("Rust + Python");
                if let Some(registry) = &self.registry {
                    ui.separator();
                    ui.label(format!("{} operations", registry.operations().len()));
                }
            });
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Status: {}", self.last_status));
                if let Some(spec) = &selected_spec {
                    ui.separator();
                    ui.label(format!(
                        "{} · {} · {}",
                        spec.category, spec.backend, spec.safety
                    ));
                }
            });
        });

        egui::SidePanel::left("operations")
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| {
                ui.heading("工具");
                ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text("搜索，例如 base64 / 请求包 / jwt"),
                );
                ui.separator();

                if let Some(registry) = &self.registry {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for op in registry.search(&self.query) {
                            let selected = self.selected_operation.as_deref() == Some(&op.id);
                            if ui
                                .selectable_label(
                                    selected,
                                    format!("{}\n{} · {}", op.name_zh, op.id, op.priority),
                                )
                                .clicked()
                            {
                                self.selected_operation = Some(op.id.clone());
                            }
                        }
                    });
                } else {
                    ui.colored_label(egui::Color32::RED, "注册表加载失败");
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("输入");
                if let Some(spec) = &selected_spec {
                    ui.label(&spec.id);
                    ui.label(format!("input: {}", spec.input.join("/")));
                    ui.label(format!("output: {}", spec.output.join("/")));
                }
            });
            ui.add(
                egui::TextEdit::multiline(&mut self.input)
                    .desired_rows(10)
                    .desired_width(f32::INFINITY),
            );

            ui.horizontal(|ui| {
                if ui.button("执行").clicked() {
                    self.run_selected();
                }
                if ui.button("清空输入").clicked() {
                    self.input.clear();
                }
                if ui.button("复制结果").clicked() && !self.output.is_empty() {
                    ctx.copy_text(self.output.clone());
                    self.last_status = "Copied".to_string();
                }
            });

            if !self.warnings.is_empty() {
                ui.separator();
                for warning in &self.warnings {
                    ui.colored_label(egui::Color32::YELLOW, warning);
                }
            }

            ui.separator();
            ui.heading("结果");
            ui.add(
                egui::TextEdit::multiline(&mut self.output)
                    .desired_rows(16)
                    .desired_width(f32::INFINITY),
            );
        });
    }
}
