use ctf_core::{OperationInput, OperationRegistry, OperationRequest, OperationSpec, TaskLimits};
use eframe::egui;

#[derive(Debug, Clone)]
struct OperationDragPayload {
    operation_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Language {
    English,
    Chinese,
}

impl Language {
    fn app_subtitle(self) -> &'static str {
        match self {
            Self::English => "Rust + Python local workbench",
            Self::Chinese => "Rust + Python 本地工作台",
        }
    }

    fn settings(self) -> &'static str {
        match self {
            Self::English => "Settings",
            Self::Chinese => "设置",
        }
    }

    fn language_label(self) -> &'static str {
        match self {
            Self::English => "Language",
            Self::Chinese => "语言",
        }
    }

    fn tools_count(self, count: usize) -> String {
        match self {
            Self::English => format!("{count} tools"),
            Self::Chinese => format!("{count} 个工具"),
        }
    }

    fn steps_count(self, count: usize) -> String {
        match self {
            Self::English => format!("{count} steps"),
            Self::Chinese => format!("{count} 步"),
        }
    }

    fn operation_name(self, spec: &OperationSpec) -> &str {
        match self {
            Self::English => spec.name_en.as_str(),
            Self::Chinese => spec.name_zh.as_str(),
        }
    }

    fn operation_item_name(self, item: &OperationListItem) -> &str {
        match self {
            Self::English => item.name_en.as_str(),
            Self::Chinese => item.name_zh.as_str(),
        }
    }

    fn categories_heading(self) -> &'static str {
        match self {
            Self::English => "Categories",
            Self::Chinese => "功能分类",
        }
    }

    fn registry_load_failed(self) -> &'static str {
        match self {
            Self::English => "Operation registry failed to load",
            Self::Chinese => "注册表加载失败",
        }
    }

    fn search_hint(self) -> &'static str {
        match self {
            Self::English => "Search, e.g. base64 / raw request / jwt",
            Self::Chinese => "搜索，例如 base64 / 请求包 / jwt",
        }
    }

    fn clear_search(self) -> &'static str {
        match self {
            Self::English => "Clear search",
            Self::Chinese => "清除搜索",
        }
    }

    fn recipe_heading(self) -> &'static str {
        match self {
            Self::English => "Recipe",
            Self::Chinese => "配方链",
        }
    }

    fn run_recipe(self) -> &'static str {
        match self {
            Self::English => "Run recipe",
            Self::Chinese => "运行配方",
        }
    }

    fn add_selected(self) -> &'static str {
        match self {
            Self::English => "Add selected",
            Self::Chinese => "添加选中",
        }
    }

    fn clear_recipe(self) -> &'static str {
        match self {
            Self::English => "Clear recipe",
            Self::Chinese => "清空配方",
        }
    }

    fn drop_tools(self) -> &'static str {
        match self {
            Self::English => "Drop tools here",
            Self::Chinese => "拖入工具",
        }
    }

    fn add_to_recipe(self) -> &'static str {
        match self {
            Self::English => "Add to recipe",
            Self::Chinese => "加入配方链",
        }
    }

    fn current_tool(self) -> &'static str {
        match self {
            Self::English => "Current tool",
            Self::Chinese => "当前工具",
        }
    }

    fn output_label(self) -> &'static str {
        match self {
            Self::English => "Output",
            Self::Chinese => "输出",
        }
    }

    fn input_heading(self) -> &'static str {
        match self {
            Self::English => "Input",
            Self::Chinese => "输入",
        }
    }

    fn run_selected(self) -> &'static str {
        match self {
            Self::English => "Run selected",
            Self::Chinese => "运行选中",
        }
    }

    fn clear_input(self) -> &'static str {
        match self {
            Self::English => "Clear input",
            Self::Chinese => "清空输入",
        }
    }

    fn copy_result(self) -> &'static str {
        match self {
            Self::English => "Copy result",
            Self::Chinese => "复制结果",
        }
    }

    fn result_heading(self) -> &'static str {
        match self {
            Self::English => "Result",
            Self::Chinese => "结果",
        }
    }

    fn no_operation_selected(self) -> &'static str {
        match self {
            Self::English => "No operation selected",
            Self::Chinese => "未选择工具",
        }
    }

    fn no_registry_loaded(self) -> &'static str {
        match self {
            Self::English => "No operation registry loaded",
            Self::Chinese => "未加载工具注册表",
        }
    }

    fn no_enabled_steps(self) -> &'static str {
        match self {
            Self::English => "No enabled steps",
            Self::Chinese => "没有启用的步骤",
        }
    }

    fn recipe_cleared(self) -> &'static str {
        match self {
            Self::English => "Recipe cleared",
            Self::Chinese => "配方已清空",
        }
    }

    fn recipe_updated(self) -> &'static str {
        match self {
            Self::English => "Recipe updated",
            Self::Chinese => "配方已更新",
        }
    }

    fn copied(self) -> &'static str {
        match self {
            Self::English => "Copied",
            Self::Chinese => "已复制",
        }
    }

    fn operation_not_found(self, operation_id: &str) -> String {
        match self {
            Self::English => format!("operation not found: {operation_id}"),
            Self::Chinese => format!("未找到工具: {operation_id}"),
        }
    }

    fn rejects_input_kind(self, operation_name: &str, current_kind: &str) -> String {
        match self {
            Self::English => {
                format!("{operation_name} does not accept previous output kind `{current_kind}`")
            }
            Self::Chinese => format!("{operation_name} 不接受上一步输出类型 `{current_kind}`"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CategoryGroup {
    id: &'static str,
    label_en: &'static str,
    label_zh: &'static str,
    hint_en: &'static str,
    hint_zh: &'static str,
    prefixes: &'static [&'static str],
}

impl CategoryGroup {
    fn label(self, language: Language) -> &'static str {
        match language {
            Language::English => self.label_en,
            Language::Chinese => self.label_zh,
        }
    }

    fn hint(self, language: Language) -> &'static str {
        match language {
            Language::English => self.hint_en,
            Language::Chinese => self.hint_zh,
        }
    }
}

#[derive(Debug, Clone)]
struct OperationListItem {
    id: String,
    name_en: String,
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
    language: Language,
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
            language: Language::English,
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
            self.output = self.language.no_operation_selected().to_string();
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
            self.output = self.language.no_registry_loaded().to_string();
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
                self.output = self.language.operation_not_found(&operation_id);
                self.last_status = "Error".to_string();
                self.trace = trace;
                self.warnings = warnings;
                return;
            };
            let Some(step_input_kind) = chain_input_kind(&spec, &current_kind) else {
                self.recipe[index].last_status = "Error".to_string();
                let operation_name = self.language.operation_name(&spec);
                self.output = self
                    .language
                    .rejects_input_kind(operation_name, &current_kind);
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
                        self.language.operation_name(&spec),
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
            self.language.no_enabled_steps()
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
            self.output = self.language.operation_not_found(operation_id);
            return;
        }

        self.recipe.push(RecipeStep::new(operation_id));
        self.last_status = self.language.recipe_updated().to_string();
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
            self.last_status = self.language.recipe_updated().to_string();
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
                name_en: op.name_en.clone(),
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
        let language = self.language;
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
                ui.label(language.app_subtitle());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.menu_button(self.language.settings(), |ui| {
                        ui.label(self.language.language_label());
                        ui.separator();
                        ui.selectable_value(&mut self.language, Language::English, "English");
                        ui.selectable_value(&mut self.language, Language::Chinese, "中文");
                    });
                    ui.separator();
                    ui.label(format!("Status: {}", self.last_status));
                    if let Some(registry) = &self.registry {
                        ui.separator();
                        ui.label(language.tools_count(registry.operations().len()));
                    }
                });
            });
            ui.add_space(4.0);
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(active_group.label(language));
                ui.separator();
                ui.label(active_group.hint(language));
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
                ui.heading(language.categories_heading());
                ui.add_space(6.0);
                if self.registry.is_some() {
                    for (group, count) in categories.iter().zip(category_counts.iter()) {
                        let selected = self.active_category == group.id;
                        let response = ui.selectable_label(
                            selected,
                            format!(
                                "{}\n{}",
                                group.label(language),
                                language.tools_count(*count)
                            ),
                        );
                        if response.clicked() {
                            self.select_category(group.id);
                        }
                        response.on_hover_text(group.hint(language));
                    }
                } else {
                    ui.colored_label(egui::Color32::RED, language.registry_load_failed());
                }
            });

        egui::SidePanel::left("operations")
            .resizable(true)
            .default_width(330.0)
            .show(ctx, |ui| {
                ui.heading(active_group.label(language));
                ui.label(active_group.hint(language));
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.query).hint_text(language.search_hint()),
                );
                ui.separator();

                if self.registry.is_some() {
                    ui.horizontal(|ui| {
                        ui.label(language.tools_count(visible_operations.len()));
                        if !self.query.is_empty() && ui.button(language.clear_search()).clicked() {
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
                                    egui::RichText::new(category_title(&op.category, language))
                                        .strong()
                                        .color(egui::Color32::from_rgb(150, 170, 190)),
                                );
                            }
                            let selected = self.selected_operation.as_deref() == Some(&op.id);
                            let label = format!(
                                "{}\n{} · {} · {}",
                                language.operation_item_name(op),
                                op.id,
                                op.priority,
                                op.safety
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
                                if ui
                                    .small_button("+")
                                    .on_hover_text(language.add_to_recipe())
                                    .clicked()
                                {
                                    operation_to_add = Some(op.id.clone());
                                }
                            });
                        }
                    });
                    if let Some(operation_id) = operation_to_add {
                        self.add_operation_to_recipe(&operation_id);
                    }
                } else {
                    ui.colored_label(egui::Color32::RED, language.registry_load_failed());
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(360.0);
                    ui.horizontal(|ui| {
                        ui.heading(language.recipe_heading());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(language.steps_count(self.recipe.len()));
                        });
                    });

                    ui.horizontal_wrapped(|ui| {
                        if ui.button(language.run_recipe()).clicked() {
                            self.run_recipe();
                        }
                        if let Some(operation_id) = self.selected_operation.clone()
                            && ui.button(language.add_selected()).clicked()
                        {
                            self.add_operation_to_recipe(&operation_id);
                        }
                        if ui.button(language.clear_recipe()).clicked() {
                            self.recipe.clear();
                            self.trace.clear();
                            self.last_status = self.language.recipe_cleared().to_string();
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
                                        egui::RichText::new(language.drop_tools())
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
                                            .map(|spec| language.operation_name(spec))
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
                        self.last_status = self.language.recipe_updated().to_string();
                    }
                    if let Some(index) = remove_step {
                        self.recipe.remove(index);
                        self.last_status = self.language.recipe_updated().to_string();
                    }

                    ui.separator();
                    if let Some(spec) = &selected_spec {
                        ui.label(egui::RichText::new(language.current_tool()).strong());
                        ui.label(language.operation_name(spec));
                        ui.label(
                            egui::RichText::new(format!(
                                "{} · {} · {}",
                                spec.id, spec.backend, spec.safety
                            ))
                            .small()
                            .color(egui::Color32::from_rgb(150, 164, 180)),
                        );
                        ui.horizontal(|ui| {
                            ui.label(format!(
                                "{}: {}",
                                language.output_label(),
                                spec.output.join(" / ")
                            ));
                            ui.separator();
                            ui.label(category_title(&spec.category, language));
                        });
                    }
                });

                ui.separator();

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading(language.input_heading());
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
                        if ui.button(language.run_recipe()).clicked() {
                            self.run_recipe();
                        }
                        if ui.button(language.run_selected()).clicked() {
                            self.run_selected();
                        }
                        if ui.button(language.clear_input()).clicked() {
                            if self.input_kind == "file" {
                                self.file_path.clear();
                            } else {
                                self.input.clear();
                            }
                        }
                        if ui.button(language.copy_result()).clicked() && !self.output.is_empty() {
                            ctx.copy_text(self.output.clone());
                            self.last_status = self.language.copied().to_string();
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
                        ui.heading(language.result_heading());
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
            label_en: "All Tools",
            label_zh: "全部工具",
            hint_en: "Show every registered operation",
            hint_zh: "显示所有已注册工具",
            prefixes: &[],
        },
        CategoryGroup {
            id: "codecs",
            label_en: "Codecs",
            label_zh: "编码转换",
            hint_en: "Base, URL, HTML, Unicode, radix, Morse, and auto decode",
            hint_zh: "Base、URL、HTML、Unicode、进制、摩斯和自动解码",
            prefixes: &["codecs.", "program.auto", "program.esolang"],
        },
        CategoryGroup {
            id: "crypto",
            label_en: "Hash & Crypto",
            label_zh: "哈希与密码",
            hint_en: "Hashes, classical ciphers, XOR, and password analysis",
            hint_zh: "哈希、古典密码、XOR 和口令相关分析",
            prefixes: &["crypto."],
        },
        CategoryGroup {
            id: "web",
            label_en: "Web / HTTP / JWT",
            label_zh: "Web / HTTP / JWT",
            hint_en: "Raw requests, code generation, JWT, and asset triage",
            hint_zh: "请求包解析、代码生成、JWT 和资产分拣",
            prefixes: &["web."],
        },
        CategoryGroup {
            id: "forensics",
            label_en: "Files & Stego",
            label_zh: "文件与隐写",
            hint_en: "Hex, entropy, image Base64, GIF frames, and local forensics",
            hint_zh: "Hex、熵、图片 Base64、GIF 分帧等本地取证工具",
            prefixes: &["file.", "forensics.image"],
        },
        CategoryGroup {
            id: "pcap",
            label_en: "PCAP / USB",
            label_zh: "PCAP / USB",
            hint_en: "HTTP, DNS, ICMP, TCP streams, and USB HID analysis",
            hint_zh: "HTTP、DNS、ICMP、TCP 流和 USB HID 分析",
            prefixes: &["forensics.pcap", "forensics.usb"],
        },
        CategoryGroup {
            id: "pwn",
            label_en: "Pwn",
            label_zh: "Pwn",
            hint_en: "cyclic, pack/unpack, shellcode assembly and disassembly",
            hint_zh: "cyclic、pack/unpack、shellcode 汇编与反汇编",
            prefixes: &["pwn."],
        },
        CategoryGroup {
            id: "reverse",
            label_en: "Reverse",
            label_zh: "逆向分析",
            hint_en: "Binary info and strings extraction",
            hint_zh: "二进制信息和 strings 提取",
            prefixes: &["reverse."],
        },
        CategoryGroup {
            id: "workspace",
            label_en: "Workspace",
            label_zh: "题目工作台",
            hint_en: "Challenge notes and writeup templates",
            hint_zh: "题目记录和 writeup 生成入口",
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

fn category_title(category: &str, language: Language) -> &str {
    match (category, language) {
        ("codecs.base", Language::English) => "Base Encoding",
        ("codecs.base", Language::Chinese) => "Base 编码",
        ("codecs.binary", Language::English) => "Binary Encoding",
        ("codecs.binary", Language::Chinese) => "二进制编码",
        ("codecs.number", Language::English) => "Radix Conversion",
        ("codecs.number", Language::Chinese) => "进制转换",
        ("codecs.text", Language::English) => "Text Encoding",
        ("codecs.text", Language::Chinese) => "文本编码",
        ("codecs.web", Language::English) => "Web Encoding",
        ("codecs.web", Language::Chinese) => "Web 编码",
        ("crypto.classical", Language::English) => "Classical Ciphers",
        ("crypto.classical", Language::Chinese) => "古典密码",
        ("crypto.hash", Language::English) => "Hashes",
        ("crypto.hash", Language::Chinese) => "哈希",
        ("crypto.xor", _) => "XOR",
        ("file.inspect", Language::English) => "File Inspection",
        ("file.inspect", Language::Chinese) => "文件检查",
        ("forensics.image", Language::English) => "Image Stego",
        ("forensics.image", Language::Chinese) => "图片隐写",
        ("forensics.pcap", _) => "PCAP",
        ("forensics.usb", _) => "USB",
        ("program.auto", Language::English) => "Auto Analysis",
        ("program.auto", Language::Chinese) => "自动分析",
        ("program.esolang", _) => "Esolang",
        ("pwn.binary", Language::English) => "Pwn Binary",
        ("pwn.binary", Language::Chinese) => "Pwn 二进制",
        ("pwn.pattern", _) => "Pwn Pattern",
        ("pwn.shellcode", _) => "Shellcode",
        ("reverse.binary", Language::English) => "Reverse Binary",
        ("reverse.binary", Language::Chinese) => "逆向二进制",
        ("web.assets", Language::English) => "Asset Triage",
        ("web.assets", Language::Chinese) => "资产分拣",
        ("web.http", _) => "HTTP",
        ("web.http.codegen", Language::English) => "Request Codegen",
        ("web.http.codegen", Language::Chinese) => "请求代码生成",
        ("web.token", _) => "Token / JWT",
        ("workspace.challenge", Language::English) => "Challenge Workspace",
        ("workspace.challenge", Language::Chinese) => "题目工作台",
        _ => category,
    }
}
