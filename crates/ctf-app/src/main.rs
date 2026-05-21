use ctf_core::{OperationInput, OperationRegistry, OperationRequest, OperationRunner, TaskLimits};
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
        }
    }
}

impl eframe::App for CtfToolsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("CTF Tools");
                ui.label("Rust + Python");
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
            ui.horizontal(|ui| {
                ui.heading("输入");
                if let Some(op) = &self.selected_operation {
                    ui.label(op);
                }
            });
            ui.add(
                egui::TextEdit::multiline(&mut self.input)
                    .desired_rows(10)
                    .desired_width(f32::INFINITY),
            );

            if ui.button("执行").clicked()
                && let (Some(registry), Some(operation)) =
                    (self.registry.clone(), self.selected_operation.clone())
            {
                let runner = OperationRunner::new(registry);
                let result = runner.run(OperationRequest {
                    operation,
                    input: OperationInput {
                        kind: "text".to_string(),
                        value: self.input.clone(),
                    },
                    limits: TaskLimits::default(),
                });
                self.output = match result {
                    Ok(response) => response
                        .outputs
                        .iter()
                        .map(|output| output.value.clone())
                        .collect::<Vec<_>>()
                        .join("\n"),
                    Err(error) => error.to_string(),
                };
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
