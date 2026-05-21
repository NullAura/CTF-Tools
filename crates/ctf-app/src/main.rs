use ctf_core::{OperationInput, OperationRegistry, OperationRequest, OperationSpec, TaskLimits};
use eframe::egui;

#[derive(Debug, Clone)]
struct OperationDragPayload {
    operation_id: String,
}

#[derive(Debug, Clone, Copy)]
struct CategoryGroup {
    id: &'static str,
    label: &'static str,
    hint: &'static str,
    prefixes: &'static [&'static str],
}

#[derive(Debug, Clone)]
struct OperationListItem {
    id: String,
    name_zh: String,
    category: String,
    priority: String,
    safety: String,
    input: Vec<String>,
}

#[derive(Debug, Clone)]
struct RecipeStep {
    operation_id: String,
    enabled: bool,
    last_status: String,
    output_preview: String,
}

impl RecipeStep {
    fn new(operation_id: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            enabled: true,
            last_status: "Ready".to_string(),
            output_preview: String::new(),
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 820.0]),
        ..Default::default()
    };

    eframe::run_native(
        "CTF Tools",
        options,
        Box::new(|cc| {
            install_cjk_font(&cc.egui_ctx);
            install_style(&cc.egui_ctx);
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

fn install_style(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(18, 20, 24);
    visuals.window_fill = egui::Color32::from_rgb(20, 23, 28);
    visuals.extreme_bg_color = egui::Color32::from_rgb(8, 10, 13);
    visuals.selection.bg_fill = egui::Color32::from_rgb(36, 112, 160);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(35, 39, 47);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(46, 53, 63);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(45, 122, 170);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(6);
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);
    ctx.set_style(style);
}

struct CtfToolsApp {
    registry: Option<OperationRegistry>,
    query: String,
    active_category: String,
    selected_operation: Option<String>,
    recipe: Vec<RecipeStep>,
    input_kind: String,
    input: String,
    file_path: String,
    output: String,
    trace: Vec<String>,
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
        let recipe = selected_operation
            .iter()
            .map(RecipeStep::new)
            .collect::<Vec<_>>();

        Self {
            registry,
            query: String::new(),
            active_category: "all".to_string(),
            selected_operation,
            recipe,
            input_kind: "text".to_string(),
            input: "ZmxhZ3t0ZXN0fQ==".to_string(),
            file_path: String::new(),
            output: String::new(),
            trace: Vec::new(),
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
        let input_value = if self.input_kind == "file" {
            self.file_path.clone()
        } else {
            self.input.clone()
        };
        let result = runner.run(OperationRequest {
            operation,
            input: OperationInput {
                kind: self.input_kind.clone(),
                value: input_value,
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

    fn run_recipe(&mut self) {
        if self.recipe.is_empty() {
            self.run_selected();
            return;
        }

        let Some(registry) = self.registry.clone() else {
            self.output = "No operation registry loaded".to_string();
            self.last_status = "Error".to_string();
            return;
        };

        let runner = ctf_runner::default_runner()
            .unwrap_or_else(|_| ctf_core::OperationRunner::new(registry.clone()));
        let mut current_kind = self.input_kind.clone();
        let mut current_value = if self.input_kind == "file" {
            self.file_path.clone()
        } else {
            self.input.clone()
        };
        let mut trace = Vec::new();
        let mut warnings = Vec::new();
        let mut ran_any_step = false;

        for step in &mut self.recipe {
            step.last_status = if step.enabled {
                "Pending".to_string()
            } else {
                "Skipped".to_string()
            };
            step.output_preview.clear();
        }

        for index in 0..self.recipe.len() {
            if !self.recipe[index].enabled {
                continue;
            }

            ran_any_step = true;
            let operation_id = self.recipe[index].operation_id.clone();
            let Some(spec) = registry.find(&operation_id).cloned() else {
                self.recipe[index].last_status = "Error".to_string();
                self.output = format!("operation not found: {operation_id}");
                self.last_status = "Error".to_string();
                self.trace = trace;
                self.warnings = warnings;
                return;
            };
            let Some(step_input_kind) = chain_input_kind(&spec, &current_kind) else {
                self.recipe[index].last_status = "Error".to_string();
                self.output = format!("{} 不接受上一步输出类型 `{}`", spec.name_zh, current_kind);
                self.last_status = "Error".to_string();
                self.trace = trace;
                self.warnings = warnings;
                return;
            };

            let result = runner.run(OperationRequest {
                operation: operation_id.clone(),
                input: OperationInput {
                    kind: step_input_kind.clone(),
                    value: current_value.clone(),
                },
                limits: TaskLimits::default(),
            });

            match result {
                Ok(response) => {
                    let joined = join_output_values(&response.outputs);
                    current_value = joined.clone();
                    current_kind = "text".to_string();
                    self.recipe[index].last_status = "OK".to_string();
                    self.recipe[index].output_preview = preview_text(&joined);
                    warnings.extend(response.warnings);
                    trace.push(format!(
                        "{}. {} -> {} bytes",
                        trace.len() + 1,
                        spec.name_zh,
                        joined.len()
                    ));
                }
                Err(error) => {
                    self.recipe[index].last_status = "Error".to_string();
                    self.recipe[index].output_preview = error.to_string();
                    self.output = error.to_string();
                    self.last_status = "Error".to_string();
                    self.trace = trace;
                    self.warnings = warnings;
                    return;
                }
            }
        }

        self.output = current_value;
        self.trace = trace;
        self.warnings = warnings;
        self.last_status = if ran_any_step {
            "OK"
        } else {
            "No enabled steps"
        }
        .to_string();
    }

    fn add_operation_to_recipe(&mut self, operation_id: &str) {
        if self
            .registry
            .as_ref()
            .and_then(|registry| registry.find(operation_id))
            .is_none()
        {
            self.last_status = "Error".to_string();
            self.output = format!("operation not found: {operation_id}");
            return;
        }

        self.recipe.push(RecipeStep::new(operation_id));
        self.last_status = "Recipe updated".to_string();
    }

    fn move_recipe_step(&mut self, index: usize, direction: isize) {
        let target = if direction.is_negative() {
            index.checked_sub(direction.unsigned_abs())
        } else {
            index.checked_add(direction as usize)
        };

        let Some(target) = target else {
            return;
        };
        if target < self.recipe.len() {
            self.recipe.swap(index, target);
            self.last_status = "Recipe updated".to_string();
        }
    }

    fn select_category(&mut self, category_id: &str) {
        self.active_category = category_id.to_string();
        let Some(registry) = &self.registry else {
            return;
        };
        let current_visible = self
            .selected_operation
            .as_deref()
            .and_then(|id| registry.find(id))
            .map(|op| operation_in_category(op.category.as_str(), category_id))
            .unwrap_or(false);
        if current_visible {
            return;
        }
        if let Some(op) = registry
            .operations()
            .iter()
            .find(|op| operation_in_category(op.category.as_str(), category_id))
        {
            self.selected_operation = Some(op.id.clone());
            self.input_kind = op
                .input
                .first()
                .cloned()
                .unwrap_or_else(|| "text".to_string());
        }
    }

    fn visible_operations(&self) -> Vec<OperationListItem> {
        let Some(registry) = &self.registry else {
            return Vec::new();
        };
        let mut operations = registry
            .search(&self.query)
            .into_iter()
            .filter(|op| operation_in_category(op.category.as_str(), &self.active_category))
            .map(|op| OperationListItem {
                id: op.id.clone(),
                name_zh: op.name_zh.clone(),
                category: op.category.clone(),
                priority: op.priority.clone(),
                safety: op.safety.clone(),
                input: op.input.clone(),
            })
            .collect::<Vec<_>>();
        operations.sort_by(|a, b| {
            category_sort_key(&a.category)
                .cmp(&category_sort_key(&b.category))
                .then_with(|| a.priority.cmp(&b.priority))
                .then_with(|| a.id.cmp(&b.id))
        });
        operations
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
        let categories = category_groups();
        let category_counts = self
            .registry
            .as_ref()
            .map(|registry| {
                categories
                    .iter()
                    .map(|group| {
                        registry
                            .operations()
                            .iter()
                            .filter(|op| operation_in_category(op.category.as_str(), group.id))
                            .count()
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let visible_operations = self.visible_operations();
        let active_group = categories
            .iter()
            .find(|group| group.id == self.active_category)
            .unwrap_or(&categories[0]);

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading("CTF Tools");
                ui.label("Rust + Python 本地工作台");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Status: {}", self.last_status));
                    if let Some(registry) = &self.registry {
                        ui.separator();
                        ui.label(format!("{} 个工具", registry.operations().len()));
                    }
                });
            });
            ui.add_space(4.0);
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(active_group.label);
                ui.separator();
                ui.label(active_group.hint);
                if let Some(spec) = &selected_spec {
                    ui.separator();
                    ui.label(format!(
                        "{} · {} · {}",
                        spec.category, spec.backend, spec.safety
                    ));
                }
            });
        });

        egui::SidePanel::left("categories")
            .resizable(false)
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.heading("功能分类");
                ui.add_space(6.0);
                if self.registry.is_some() {
                    for (group, count) in categories.iter().zip(category_counts.iter()) {
                        let selected = self.active_category == group.id;
                        let response = ui.selectable_label(
                            selected,
                            format!("{}\n{} 个工具", group.label, count),
                        );
                        if response.clicked() {
                            self.select_category(group.id);
                        }
                        response.on_hover_text(group.hint);
                    }
                } else {
                    ui.colored_label(egui::Color32::RED, "注册表加载失败");
                }
            });

        egui::SidePanel::left("operations")
            .resizable(true)
            .default_width(330.0)
            .show(ctx, |ui| {
                ui.heading(active_group.label);
                ui.label(active_group.hint);
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text("搜索，例如 base64 / 请求包 / jwt"),
                );
                ui.separator();

                if self.registry.is_some() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} 个工具", visible_operations.len()));
                        if !self.query.is_empty() && ui.button("清除搜索").clicked() {
                            self.query.clear();
                        }
                    });
                    ui.separator();
                    let mut previous_category = String::new();
                    let mut operation_to_add = None;
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for op in &visible_operations {
                            if previous_category != op.category {
                                previous_category = op.category.clone();
                                ui.add_space(6.0);
                                ui.label(
                                    egui::RichText::new(category_title(&op.category))
                                        .strong()
                                        .color(egui::Color32::from_rgb(150, 170, 190)),
                                );
                            }
                            let selected = self.selected_operation.as_deref() == Some(&op.id);
                            let label = format!(
                                "{}\n{} · {} · {}",
                                op.name_zh, op.id, op.priority, op.safety
                            );
                            ui.horizontal(|ui| {
                                let drag_source = ui.dnd_drag_source(
                                    egui::Id::new(("operation-drag", &op.id)),
                                    OperationDragPayload {
                                        operation_id: op.id.clone(),
                                    },
                                    |ui| ui.selectable_label(selected, label),
                                );
                                if drag_source.inner.clicked() {
                                    self.selected_operation = Some(op.id.clone());
                                    if !op.input.iter().any(|kind| kind == &self.input_kind) {
                                        self.input_kind = op
                                            .input
                                            .first()
                                            .cloned()
                                            .unwrap_or_else(|| "text".to_string());
                                    }
                                }
                                if ui.small_button("+").on_hover_text("加入配方链").clicked() {
                                    operation_to_add = Some(op.id.clone());
                                }
                            });
                        }
                    });
                    if let Some(operation_id) = operation_to_add {
                        self.add_operation_to_recipe(&operation_id);
                    }
                } else {
                    ui.colored_label(egui::Color32::RED, "注册表加载失败");
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(360.0);
                    ui.horizontal(|ui| {
                        ui.heading("配方链");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("{} 步", self.recipe.len()));
                        });
                    });

                    ui.horizontal_wrapped(|ui| {
                        if ui.button("运行配方").clicked() {
                            self.run_recipe();
                        }
                        if let Some(operation_id) = self.selected_operation.clone()
                            && ui.button("添加选中").clicked()
                        {
                            self.add_operation_to_recipe(&operation_id);
                        }
                        if ui.button("清空配方").clicked() {
                            self.recipe.clear();
                            self.trace.clear();
                            self.last_status = "Recipe cleared".to_string();
                        }
                    });

                    let mut move_step = None;
                    let mut remove_step = None;
                    let mut duplicate_step = None;
                    let drop_frame = egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::same(8))
                        .fill(egui::Color32::from_rgb(13, 16, 20));
                    let (_inner, dropped_operation) =
                        ui.dnd_drop_zone::<OperationDragPayload, _>(drop_frame, |ui| {
                            ui.set_min_height(300.0);
                            if self.recipe.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(110.0);
                                    ui.label(
                                        egui::RichText::new("拖入工具")
                                            .color(egui::Color32::from_rgb(120, 136, 154)),
                                    );
                                });
                            } else {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    for (index, step) in self.recipe.iter_mut().enumerate() {
                                        let spec = self
                                            .registry
                                            .as_ref()
                                            .and_then(|registry| registry.find(&step.operation_id));
                                        let name = spec
                                            .map(|spec| spec.name_zh.as_str())
                                            .unwrap_or(step.operation_id.as_str());
                                        let detail = spec
                                            .map(|spec| {
                                                format!(
                                                    "{} · {} · {}",
                                                    spec.id, spec.priority, spec.safety
                                                )
                                            })
                                            .unwrap_or_else(|| step.operation_id.clone());

                                        ui.horizontal(|ui| {
                                            ui.checkbox(&mut step.enabled, "");
                                            ui.label(
                                                egui::RichText::new(format!("{}.", index + 1))
                                                    .color(egui::Color32::from_rgb(130, 145, 160)),
                                            );
                                            ui.vertical(|ui| {
                                                ui.label(egui::RichText::new(name).strong());
                                                ui.label(
                                                    egui::RichText::new(detail).small().color(
                                                        egui::Color32::from_rgb(145, 158, 174),
                                                    ),
                                                );
                                                if !step.output_preview.is_empty() {
                                                    ui.label(
                                                        egui::RichText::new(&step.output_preview)
                                                            .small()
                                                            .monospace()
                                                            .color(egui::Color32::from_rgb(
                                                                178, 188, 198,
                                                            )),
                                                    );
                                                }
                                            });
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui.small_button("×").clicked() {
                                                        remove_step = Some(index);
                                                    }
                                                    if ui.small_button("⧉").clicked() {
                                                        duplicate_step = Some(index);
                                                    }
                                                    if ui.small_button("↓").clicked() {
                                                        move_step = Some((index, 1));
                                                    }
                                                    if ui.small_button("↑").clicked() {
                                                        move_step = Some((index, -1));
                                                    }
                                                    ui.label(
                                                        egui::RichText::new(&step.last_status)
                                                            .small()
                                                            .color(status_color(&step.last_status)),
                                                    );
                                                },
                                            );
                                        });
                                        ui.separator();
                                    }
                                });
                            }
                        });

                    if let Some(payload) = dropped_operation {
                        self.add_operation_to_recipe(&payload.operation_id);
                    }
                    if let Some((index, direction)) = move_step {
                        self.move_recipe_step(index, direction);
                    }
                    if let Some(index) = duplicate_step
                        && let Some(step) = self.recipe.get(index).cloned()
                    {
                        self.recipe.insert(index + 1, step);
                        self.last_status = "Recipe updated".to_string();
                    }
                    if let Some(index) = remove_step {
                        self.recipe.remove(index);
                        self.last_status = "Recipe updated".to_string();
                    }

                    ui.separator();
                    if let Some(spec) = &selected_spec {
                        ui.label(egui::RichText::new("当前工具").strong());
                        ui.label(&spec.name_zh);
                        ui.label(
                            egui::RichText::new(format!(
                                "{} · {} · {}",
                                spec.id, spec.backend, spec.safety
                            ))
                            .small()
                            .color(egui::Color32::from_rgb(150, 164, 180)),
                        );
                        ui.horizontal(|ui| {
                            ui.label(format!("输出: {}", spec.output.join(" / ")));
                            ui.separator();
                            ui.label(category_title(&spec.category));
                        });
                    }
                });

                ui.separator();

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading("输入");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            for kind in ["file", "bytes", "text"] {
                                ui.selectable_value(&mut self.input_kind, kind.to_string(), kind);
                            }
                        });
                    });

                    if self.input_kind == "file" {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.file_path)
                                .hint_text("/path/to/file")
                                .desired_width(f32::INFINITY),
                        );
                    } else {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.input)
                                .font(egui::TextStyle::Monospace)
                                .desired_rows(12)
                                .desired_width(f32::INFINITY),
                        );
                    }

                    ui.horizontal(|ui| {
                        if ui.button("运行配方").clicked() {
                            self.run_recipe();
                        }
                        if ui.button("运行选中").clicked() {
                            self.run_selected();
                        }
                        if ui.button("清空输入").clicked() {
                            if self.input_kind == "file" {
                                self.file_path.clear();
                            } else {
                                self.input.clear();
                            }
                        }
                        if ui.button("复制结果").clicked() && !self.output.is_empty() {
                            ctx.copy_text(self.output.clone());
                            self.last_status = "Copied".to_string();
                        }
                    });

                    if !self.trace.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            for item in &self.trace {
                                ui.label(
                                    egui::RichText::new(item)
                                        .small()
                                        .color(egui::Color32::from_rgb(142, 168, 192)),
                                );
                            }
                        });
                    }

                    if !self.warnings.is_empty() {
                        ui.separator();
                        for warning in &self.warnings {
                            ui.colored_label(egui::Color32::YELLOW, warning);
                        }
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.heading("结果");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("{} bytes", self.output.len()));
                        });
                    });
                    ui.add(
                        egui::TextEdit::multiline(&mut self.output)
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(18)
                            .desired_width(f32::INFINITY),
                    );
                });
            });
        });
    }
}

fn chain_input_kind(spec: &OperationSpec, current_kind: &str) -> Option<String> {
    if spec.input.iter().any(|kind| kind == current_kind) {
        return Some(current_kind.to_string());
    }
    for fallback in ["text", "bytes"] {
        if spec.input.iter().any(|kind| kind == fallback) {
            return Some(fallback.to_string());
        }
    }
    spec.input.first().cloned()
}

fn join_output_values(outputs: &[ctf_core::OperationOutput]) -> String {
    outputs
        .iter()
        .map(|output| output.value.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

fn preview_text(value: &str) -> String {
    const LIMIT: usize = 96;
    let single_line = value.replace(['\r', '\n'], " ");
    let mut preview = String::new();
    for (index, character) in single_line.chars().enumerate() {
        if index >= LIMIT {
            preview.push_str("...");
            return preview;
        }
        preview.push(character);
    }
    preview
}

fn status_color(status: &str) -> egui::Color32 {
    match status {
        "OK" => egui::Color32::from_rgb(90, 190, 130),
        "Error" => egui::Color32::from_rgb(230, 105, 105),
        "Skipped" => egui::Color32::from_rgb(145, 158, 174),
        "Pending" => egui::Color32::from_rgb(230, 180, 90),
        _ => egui::Color32::from_rgb(150, 164, 180),
    }
}

fn category_groups() -> Vec<CategoryGroup> {
    vec![
        CategoryGroup {
            id: "all",
            label: "全部工具",
            hint: "显示所有已注册工具",
            prefixes: &[],
        },
        CategoryGroup {
            id: "codecs",
            label: "编码转换",
            hint: "Base、URL、HTML、Unicode、进制、摩斯和自动解码",
            prefixes: &["codecs.", "program.auto", "program.esolang"],
        },
        CategoryGroup {
            id: "crypto",
            label: "哈希与密码",
            hint: "哈希、古典密码、XOR 和口令相关分析",
            prefixes: &["crypto."],
        },
        CategoryGroup {
            id: "web",
            label: "Web / HTTP / JWT",
            hint: "请求包解析、代码生成、JWT 和资产分拣",
            prefixes: &["web."],
        },
        CategoryGroup {
            id: "forensics",
            label: "文件与隐写",
            hint: "Hex、熵、图片 Base64、GIF 分帧等本地取证工具",
            prefixes: &["file.", "forensics.image"],
        },
        CategoryGroup {
            id: "pcap",
            label: "PCAP / USB",
            hint: "HTTP、DNS、ICMP、TCP 流和 USB HID 分析",
            prefixes: &["forensics.pcap", "forensics.usb"],
        },
        CategoryGroup {
            id: "pwn",
            label: "Pwn",
            hint: "cyclic、pack/unpack、shellcode 汇编与反汇编",
            prefixes: &["pwn."],
        },
        CategoryGroup {
            id: "reverse",
            label: "逆向分析",
            hint: "二进制信息和 strings 提取",
            prefixes: &["reverse."],
        },
        CategoryGroup {
            id: "workspace",
            label: "题目工作台",
            hint: "题目记录和 writeup 生成入口",
            prefixes: &["workspace."],
        },
    ]
}

fn category_sort_key(category: &str) -> usize {
    match category {
        "codecs.base" => 10,
        "codecs.web" => 11,
        "codecs.text" => 12,
        "codecs.binary" => 13,
        "codecs.number" => 14,
        "program.auto" => 15,
        "program.esolang" => 16,
        "crypto.hash" => 20,
        "crypto.xor" => 21,
        "crypto.classical" => 22,
        "web.http" => 30,
        "web.http.codegen" => 31,
        "web.token" => 32,
        "web.assets" => 33,
        "file.inspect" => 40,
        "forensics.image" => 41,
        "forensics.pcap" => 50,
        "forensics.usb" => 51,
        "pwn.pattern" => 60,
        "pwn.binary" => 61,
        "pwn.shellcode" => 62,
        "reverse.binary" => 70,
        "workspace.challenge" => 80,
        _ => 999,
    }
}

fn operation_in_category(category: &str, group_id: &str) -> bool {
    if group_id == "all" {
        return true;
    }
    category_groups()
        .into_iter()
        .find(|group| group.id == group_id)
        .map(|group| {
            group
                .prefixes
                .iter()
                .any(|prefix| category.starts_with(prefix))
        })
        .unwrap_or(false)
}

fn category_title(category: &str) -> &str {
    match category {
        "codecs.base" => "Base 编码",
        "codecs.binary" => "二进制编码",
        "codecs.number" => "进制转换",
        "codecs.text" => "文本编码",
        "codecs.web" => "Web 编码",
        "crypto.classical" => "古典密码",
        "crypto.hash" => "哈希",
        "crypto.xor" => "XOR",
        "file.inspect" => "文件检查",
        "forensics.image" => "图片隐写",
        "forensics.pcap" => "PCAP",
        "forensics.usb" => "USB",
        "program.auto" => "自动分析",
        "program.esolang" => "Esolang",
        "pwn.binary" => "Pwn 二进制",
        "pwn.pattern" => "Pwn Pattern",
        "pwn.shellcode" => "Shellcode",
        "reverse.binary" => "逆向二进制",
        "web.assets" => "资产分拣",
        "web.http" => "HTTP",
        "web.http.codegen" => "请求代码生成",
        "web.token" => "Token / JWT",
        "workspace.challenge" => "题目工作台",
        _ => category,
    }
}
