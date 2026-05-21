use ctf_core::{OperationInput, OperationRegistry, OperationRequest, OperationSpec, TaskLimits};
use eframe::egui;

mod launcher_ui;

#[derive(Debug, Clone)]
struct OperationDragPayload {
    operation_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppWorkspace {
    Operations,
    Launcher,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AppTheme {
    Dark,
    Light,
}

impl AppTheme {
    fn from_setting(value: &str) -> Self {
        if value == "light" {
            Self::Light
        } else {
            Self::Dark
        }
    }

    pub(crate) fn as_setting(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    pub(crate) fn label(self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Dark, Language::English) => "Dark",
            (Self::Dark, Language::Chinese) => "深色",
            (Self::Light, Language::English) => "Light",
            (Self::Light, Language::Chinese) => "浅色",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Language {
    English,
    Chinese,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct UiTokens {
    pub bg: egui::Color32,
    pub panel: egui::Color32,
    pub panel_alt: egui::Color32,
    pub card: egui::Color32,
    pub card_hover: egui::Color32,
    pub card_selected: egui::Color32,
    pub border: egui::Color32,
    pub text: egui::Color32,
    pub muted: egui::Color32,
    pub dim: egui::Color32,
    pub accent: egui::Color32,
    pub accent_soft: egui::Color32,
    pub success: egui::Color32,
    pub warning: egui::Color32,
    pub danger: egui::Color32,
    pub input_bg: egui::Color32,
}

pub(crate) fn ui_tokens(theme: AppTheme) -> UiTokens {
    match theme {
        AppTheme::Dark => UiTokens {
            bg: egui::Color32::from_rgb(10, 12, 16),
            panel: egui::Color32::from_rgb(16, 19, 24),
            panel_alt: egui::Color32::from_rgb(21, 25, 32),
            card: egui::Color32::from_rgb(23, 28, 36),
            card_hover: egui::Color32::from_rgb(31, 38, 49),
            card_selected: egui::Color32::from_rgb(27, 68, 98),
            border: egui::Color32::from_rgb(49, 57, 70),
            text: egui::Color32::from_rgb(232, 237, 243),
            muted: egui::Color32::from_rgb(158, 170, 184),
            dim: egui::Color32::from_rgb(105, 116, 130),
            accent: egui::Color32::from_rgb(58, 145, 202),
            accent_soft: egui::Color32::from_rgb(23, 55, 76),
            success: egui::Color32::from_rgb(86, 179, 125),
            warning: egui::Color32::from_rgb(221, 174, 77),
            danger: egui::Color32::from_rgb(220, 92, 92),
            input_bg: egui::Color32::from_rgb(9, 12, 17),
        },
        AppTheme::Light => UiTokens {
            bg: egui::Color32::from_rgb(238, 242, 247),
            panel: egui::Color32::from_rgb(248, 250, 252),
            panel_alt: egui::Color32::from_rgb(241, 245, 249),
            card: egui::Color32::from_rgb(255, 255, 255),
            card_hover: egui::Color32::from_rgb(236, 244, 250),
            card_selected: egui::Color32::from_rgb(210, 234, 248),
            border: egui::Color32::from_rgb(202, 213, 225),
            text: egui::Color32::from_rgb(18, 24, 38),
            muted: egui::Color32::from_rgb(78, 91, 108),
            dim: egui::Color32::from_rgb(126, 139, 155),
            accent: egui::Color32::from_rgb(28, 118, 172),
            accent_soft: egui::Color32::from_rgb(219, 238, 249),
            success: egui::Color32::from_rgb(40, 145, 92),
            warning: egui::Color32::from_rgb(170, 116, 30),
            danger: egui::Color32::from_rgb(190, 63, 63),
            input_bg: egui::Color32::from_rgb(255, 255, 255),
        },
    }
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

    fn operations_workspace(self) -> &'static str {
        match self {
            Self::English => "Operations",
            Self::Chinese => "工具流水线",
        }
    }

    fn launcher_workspace(self) -> &'static str {
        match self {
            Self::English => "Launcher",
            Self::Chinese => "工具启动器",
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
    output: Vec<String>,
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
            install_style_for(&cc.egui_ctx, AppTheme::Dark);
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

pub(crate) fn install_style_for(ctx: &egui::Context, theme: AppTheme) {
    let tokens = ui_tokens(theme);
    let mut visuals = match theme {
        AppTheme::Dark => egui::Visuals::dark(),
        AppTheme::Light => egui::Visuals::light(),
    };
    visuals.panel_fill = tokens.bg;
    visuals.window_fill = tokens.panel;
    visuals.extreme_bg_color = tokens.bg;
    visuals.override_text_color = Some(tokens.text);
    visuals.selection.bg_fill = tokens.accent;
    visuals.selection.stroke.color = tokens.text;
    visuals.widgets.noninteractive.bg_fill = tokens.panel;
    visuals.widgets.noninteractive.fg_stroke.color = tokens.text;
    visuals.widgets.inactive.bg_fill = tokens.panel_alt;
    visuals.widgets.inactive.fg_stroke.color = tokens.text;
    visuals.widgets.hovered.bg_fill = tokens.card_hover;
    visuals.widgets.hovered.fg_stroke.color = tokens.text;
    visuals.widgets.active.bg_fill = tokens.accent;
    visuals.widgets.active.fg_stroke.color = tokens.text;
    visuals.widgets.open.bg_fill = tokens.card_hover;
    visuals.window_stroke.color = tokens.border;
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(11.0, 6.0);
    style.spacing.menu_margin = egui::Margin::same(8);
    style.spacing.window_margin = egui::Margin::same(10);
    style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(6);
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);
    style.visuals.widgets.open.corner_radius = egui::CornerRadius::same(6);
    ctx.set_style(style);
}

pub(crate) fn panel_frame(theme: AppTheme) -> egui::Frame {
    let tokens = ui_tokens(theme);
    egui::Frame::new()
        .fill(tokens.panel)
        .stroke(egui::Stroke::new(1.0, tokens.border))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::same(10))
}

pub(crate) fn card_frame(theme: AppTheme, selected: bool) -> egui::Frame {
    let tokens = ui_tokens(theme);
    egui::Frame::new()
        .fill(if selected {
            tokens.card_selected
        } else {
            tokens.card
        })
        .stroke(egui::Stroke::new(
            1.0,
            if selected {
                tokens.accent
            } else {
                tokens.border
            },
        ))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::same(10))
}

pub(crate) fn section_heading(
    ui: &mut egui::Ui,
    theme: AppTheme,
    title: &str,
    detail: Option<&str>,
) {
    let tokens = ui_tokens(theme);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(title)
                .strong()
                .size(14.0)
                .color(tokens.text),
        );
        if let Some(detail) = detail {
            ui.label(egui::RichText::new(detail).small().color(tokens.dim));
        }
    });
}

pub(crate) fn badge(
    ui: &mut egui::Ui,
    theme: AppTheme,
    text: impl Into<String>,
    fill: egui::Color32,
) -> egui::Response {
    let tokens = ui_tokens(theme);
    egui::Frame::new()
        .fill(fill)
        .corner_radius(egui::CornerRadius::same(5))
        .inner_margin(egui::Margin::symmetric(7, 3))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(text.into())
                    .small()
                    .strong()
                    .color(tokens.text),
            )
        })
        .inner
}

fn primary_button(ui: &mut egui::Ui, theme: AppTheme, label: &str) -> egui::Response {
    let tokens = ui_tokens(theme);
    ui.add(
        egui::Button::new(egui::RichText::new(label).strong().color(tokens.text))
            .fill(tokens.accent)
            .stroke(egui::Stroke::new(1.0, tokens.accent)),
    )
}

fn soft_button(ui: &mut egui::Ui, theme: AppTheme, label: &str) -> egui::Response {
    let tokens = ui_tokens(theme);
    ui.add(
        egui::Button::new(label)
            .fill(tokens.panel_alt)
            .stroke(egui::Stroke::new(1.0, tokens.border)),
    )
}

fn panel_toggle(ui: &mut egui::Ui, visible: &mut bool, label: &str) {
    let icon = if *visible { "-" } else { "+" };
    if ui.small_button(format!("{icon} {label}")).clicked() {
        *visible = !*visible;
    }
}

fn text(language: Language, english: &'static str, chinese: &'static str) -> &'static str {
    match language {
        Language::English => english,
        Language::Chinese => chinese,
    }
}

fn priority_fill(theme: AppTheme, priority: &str) -> egui::Color32 {
    let tokens = ui_tokens(theme);
    match priority {
        "P0" => tokens.accent,
        "P1" => tokens.accent_soft,
        _ => tokens.panel_alt,
    }
}

fn safety_fill(theme: AppTheme, safety: &str) -> egui::Color32 {
    let tokens = ui_tokens(theme);
    match safety {
        "safe" => tokens.success,
        "network" | "external" => tokens.warning,
        "dangerous" => tokens.danger,
        _ => tokens.panel_alt,
    }
}

fn status_fill(theme: AppTheme, status: &str) -> egui::Color32 {
    let tokens = ui_tokens(theme);
    match status {
        "OK" | "Ready" => tokens.success,
        "Error" => tokens.danger,
        "Pending" => tokens.warning,
        "Skipped" => tokens.panel_alt,
        _ => tokens.accent_soft,
    }
}

fn empty_state(ui: &mut egui::Ui, theme: AppTheme, title: &str, body: &str) {
    let tokens = ui_tokens(theme);
    ui.vertical_centered(|ui| {
        ui.add_space(28.0);
        ui.label(
            egui::RichText::new(title)
                .strong()
                .size(15.0)
                .color(tokens.muted),
        );
        ui.label(egui::RichText::new(body).small().color(tokens.dim));
        ui.add_space(28.0);
    });
}

struct CtfToolsApp {
    registry: Option<OperationRegistry>,
    language: Language,
    theme: AppTheme,
    workspace: AppWorkspace,
    launcher: launcher_ui::LauncherUiState,
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
    is_fullscreen: bool,
    show_categories: bool,
    show_tool_library: bool,
    show_recipe: bool,
    show_details: bool,
    show_launcher_editor: bool,
}

impl CtfToolsApp {
    fn new() -> Self {
        let registry = OperationRegistry::load_default().ok();
        let launcher = launcher_ui::LauncherUiState::load();
        let theme = AppTheme::from_setting(launcher.theme_setting());
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
            theme,
            workspace: AppWorkspace::Operations,
            launcher,
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
            is_fullscreen: false,
            show_categories: true,
            show_tool_library: true,
            show_recipe: true,
            show_details: true,
            show_launcher_editor: true,
        }
    }

    fn handle_global_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|input| input.modifiers.command && input.key_pressed(egui::Key::Comma)) {
            self.workspace = AppWorkspace::Launcher;
            self.launcher
                .set_status("Settings are available in the right panel");
        }
        if ctx.input(|input| input.modifiers.command && input.key_pressed(egui::Key::M)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
        if ctx.input(|input| input.modifiers.command && input.key_pressed(egui::Key::W)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if ctx.input(|input| input.modifiers.command && input.key_pressed(egui::Key::H)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        if ctx.input(|input| {
            input.modifiers.command && input.modifiers.ctrl && input.key_pressed(egui::Key::F)
        }) {
            self.is_fullscreen = !self.is_fullscreen;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.is_fullscreen));
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
                output: op.output.clone(),
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

    fn render_top_toolbar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, total_tools: usize) {
        let tokens = ui_tokens(self.theme);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("CTF Tools")
                    .strong()
                    .size(18.0)
                    .color(tokens.text),
            );
            ui.label(egui::RichText::new(self.language.app_subtitle()).color(tokens.muted));
            ui.separator();
            ui.selectable_value(
                &mut self.workspace,
                AppWorkspace::Operations,
                self.language.operations_workspace(),
            );
            ui.selectable_value(
                &mut self.workspace,
                AppWorkspace::Launcher,
                self.language.launcher_workspace(),
            );

            if self.workspace == AppWorkspace::Operations {
                ui.separator();
                panel_toggle(
                    ui,
                    &mut self.show_categories,
                    text(self.language, "Categories", "分类"),
                );
                panel_toggle(
                    ui,
                    &mut self.show_tool_library,
                    text(self.language, "Tools", "工具"),
                );
                panel_toggle(
                    ui,
                    &mut self.show_recipe,
                    text(self.language, "Recipe", "配方"),
                );
                panel_toggle(
                    ui,
                    &mut self.show_details,
                    text(self.language, "Details", "详情"),
                );
            } else {
                ui.separator();
                panel_toggle(
                    ui,
                    &mut self.show_launcher_editor,
                    text(self.language, "Inspector", "检查器"),
                );
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.menu_button(self.language.settings(), |ui| {
                    ui.label(self.language.language_label());
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.language, Language::English, "English");
                        ui.selectable_value(&mut self.language, Language::Chinese, "中文");
                    });
                    ui.separator();
                    ui.label(text(self.language, "Theme", "主题"));
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(self.theme == AppTheme::Dark, "Dark")
                            .clicked()
                        {
                            self.set_app_theme(ctx, AppTheme::Dark);
                            ui.close();
                        }
                        if ui
                            .selectable_label(self.theme == AppTheme::Light, "Light")
                            .clicked()
                        {
                            self.set_app_theme(ctx, AppTheme::Light);
                            ui.close();
                        }
                    });
                });
                ui.separator();
                badge(
                    ui,
                    self.theme,
                    &self.last_status,
                    status_fill(self.theme, &self.last_status),
                );
                ui.separator();
                ui.label(
                    egui::RichText::new(self.language.tools_count(total_tools)).color(tokens.muted),
                );
            });
        });
    }

    fn render_operations_workspace(&mut self, ctx: &egui::Context) {
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

        if self.show_categories || self.show_tool_library {
            egui::SidePanel::left("operations_library")
                .resizable(true)
                .default_width(400.0)
                .width_range(280.0..=520.0)
                .show(ctx, |ui| {
                    panel_frame(self.theme).show(ui, |ui| {
                        if self.show_categories {
                            self.render_category_nav(ui, &categories, &category_counts);
                        }
                        if self.show_categories && self.show_tool_library {
                            ui.separator();
                        }
                        if self.show_tool_library {
                            self.render_tool_library(ui, active_group, &visible_operations);
                        }
                    });
                });
        }

        egui::TopBottomPanel::bottom("operations_status")
            .exact_height(28.0)
            .show(ctx, |ui| {
                let tokens = ui_tokens(self.theme);
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(active_group.label(language)).color(tokens.text));
                    ui.separator();
                    ui.label(egui::RichText::new(active_group.hint(language)).color(tokens.muted));
                    if let Some(spec) = &selected_spec {
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!(
                                "{} · {} · {}",
                                spec.category, spec.backend, spec.safety
                            ))
                            .color(tokens.muted),
                        );
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if self.show_recipe {
                    ui.vertical(|ui| {
                        ui.set_width(370.0);
                        self.render_recipe_panel(ui);
                        if self.show_details {
                            ui.add_space(8.0);
                            self.render_selected_details(ui, selected_spec.as_ref());
                        }
                    });
                    ui.separator();
                }
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width().max(360.0));
                    self.render_io_panel(ui, ctx);
                });
            });
        });
    }

    fn render_category_nav(
        &mut self,
        ui: &mut egui::Ui,
        categories: &[CategoryGroup],
        category_counts: &[usize],
    ) {
        section_heading(ui, self.theme, self.language.categories_heading(), None);
        ui.add_space(4.0);
        if self.registry.is_none() {
            let tokens = ui_tokens(self.theme);
            ui.colored_label(tokens.danger, self.language.registry_load_failed());
            return;
        }

        egui::ScrollArea::vertical()
            .id_salt("operations_category_scroll")
            .max_height(190.0)
            .show(ui, |ui| {
                for (group, count) in categories.iter().zip(category_counts.iter()) {
                    let selected = self.active_category == group.id;
                    let response = card_frame(self.theme, selected).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(group.label(self.language)).strong());
                                ui.label(
                                    egui::RichText::new(group.hint(self.language))
                                        .small()
                                        .color(ui_tokens(self.theme).muted),
                                );
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    badge(
                                        ui,
                                        self.theme,
                                        count.to_string(),
                                        ui_tokens(self.theme).accent_soft,
                                    );
                                },
                            );
                        });
                    });
                    if response.response.clicked() {
                        self.select_category(group.id);
                    }
                    ui.add_space(5.0);
                }
            });
    }

    fn render_tool_library(
        &mut self,
        ui: &mut egui::Ui,
        active_group: &CategoryGroup,
        visible_operations: &[OperationListItem],
    ) {
        let tokens = ui_tokens(self.theme);
        section_heading(
            ui,
            self.theme,
            active_group.label(self.language),
            Some(&self.language.tools_count(visible_operations.len())),
        );
        ui.label(
            egui::RichText::new(active_group.hint(self.language))
                .small()
                .color(tokens.muted),
        );
        ui.add(
            egui::TextEdit::singleline(&mut self.query)
                .hint_text(self.language.search_hint())
                .desired_width(f32::INFINITY),
        );
        ui.horizontal(|ui| {
            if !self.query.is_empty()
                && soft_button(ui, self.theme, self.language.clear_search()).clicked()
            {
                self.query.clear();
            }
        });
        ui.separator();

        if self.registry.is_none() {
            ui.colored_label(tokens.danger, self.language.registry_load_failed());
            return;
        }
        if visible_operations.is_empty() {
            empty_state(
                ui,
                self.theme,
                text(self.language, "No matching operations", "没有匹配的工具"),
                text(
                    self.language,
                    "Try another keyword or category.",
                    "换一个关键词或分类试试。",
                ),
            );
            return;
        }

        let mut previous_category = String::new();
        let mut operation_to_add = None;
        egui::ScrollArea::vertical()
            .id_salt("operations_tool_library_scroll")
            .show(ui, |ui| {
                for op in visible_operations {
                    if previous_category != op.category {
                        previous_category = op.category.clone();
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(category_title(&op.category, self.language))
                                .strong()
                                .color(tokens.muted),
                        );
                    }
                    let selected = self.selected_operation.as_deref() == Some(&op.id);
                    let drag_source = ui.dnd_drag_source(
                        egui::Id::new(("operation-drag", &op.id)),
                        OperationDragPayload {
                            operation_id: op.id.clone(),
                        },
                        |ui| self.render_operation_card(ui, op, selected),
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
                    if drag_source.inner.double_clicked() {
                        operation_to_add = Some(op.id.clone());
                    }
                    ui.add_space(6.0);
                }
            });
        if let Some(operation_id) = operation_to_add {
            self.add_operation_to_recipe(&operation_id);
        }
    }

    fn render_operation_card(
        &self,
        ui: &mut egui::Ui,
        op: &OperationListItem,
        selected: bool,
    ) -> egui::Response {
        let tokens = ui_tokens(self.theme);
        card_frame(self.theme, selected)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(self.language.operation_item_name(op)).strong(),
                        );
                        ui.label(
                            egui::RichText::new(&op.id)
                                .small()
                                .monospace()
                                .color(tokens.muted),
                        );
                        ui.horizontal_wrapped(|ui| {
                            badge(
                                ui,
                                self.theme,
                                &op.priority,
                                priority_fill(self.theme, &op.priority),
                            );
                            badge(
                                ui,
                                self.theme,
                                &op.safety,
                                safety_fill(self.theme, &op.safety),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} -> {}",
                                    op.input.join("/"),
                                    op.output.join("/")
                                ))
                                .small()
                                .color(tokens.dim),
                            );
                        });
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("drag")
                                .small()
                                .color(ui_tokens(self.theme).dim),
                        )
                        .on_hover_text(self.language.add_to_recipe());
                    });
                });
            })
            .response
    }

    fn render_recipe_panel(&mut self, ui: &mut egui::Ui) {
        panel_frame(self.theme).show(ui, |ui| {
            ui.horizontal(|ui| {
                section_heading(
                    ui,
                    self.theme,
                    self.language.recipe_heading(),
                    Some(&self.language.steps_count(self.recipe.len())),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if soft_button(ui, self.theme, self.language.clear_recipe()).clicked() {
                        self.recipe.clear();
                        self.trace.clear();
                        self.last_status = self.language.recipe_cleared().to_string();
                    }
                });
            });
            ui.horizontal_wrapped(|ui| {
                if primary_button(ui, self.theme, self.language.run_recipe()).clicked() {
                    self.run_recipe();
                }
                if let Some(operation_id) = self.selected_operation.clone()
                    && soft_button(ui, self.theme, self.language.add_selected()).clicked()
                {
                    self.add_operation_to_recipe(&operation_id);
                }
            });
            ui.add_space(6.0);

            let mut move_step = None;
            let mut remove_step = None;
            let mut duplicate_step = None;
            let drop_frame = egui::Frame::new()
                .fill(ui_tokens(self.theme).input_bg)
                .stroke(egui::Stroke::new(1.0, ui_tokens(self.theme).border))
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(egui::Margin::same(8));
            let (_inner, dropped_operation) =
                ui.dnd_drop_zone::<OperationDragPayload, _>(drop_frame, |ui| {
                    ui.set_min_height(300.0);
                    if self.recipe.is_empty() {
                        empty_state(
                            ui,
                            self.theme,
                            self.language.drop_tools(),
                            text(
                                self.language,
                                "Drag tools here or use Add selected.",
                                "拖入工具，或点击添加选中。",
                            ),
                        );
                    } else {
                        egui::ScrollArea::vertical()
                            .id_salt("operations_recipe_scroll")
                            .show(ui, |ui| {
                                for (index, step) in self.recipe.iter_mut().enumerate() {
                                    let spec = self
                                        .registry
                                        .as_ref()
                                        .and_then(|registry| registry.find(&step.operation_id));
                                    let name = spec
                                        .map(|spec| self.language.operation_name(spec))
                                        .unwrap_or(step.operation_id.as_str());
                                    let detail = spec
                                        .map(|spec| {
                                            format!(
                                                "{} · {} · {}",
                                                spec.id, spec.priority, spec.safety
                                            )
                                        })
                                        .unwrap_or_else(|| step.operation_id.clone());

                                    card_frame(self.theme, false).show(ui, |ui| {
                                        let tokens = ui_tokens(self.theme);
                                        ui.set_min_width(ui.available_width().max(220.0));
                                        ui.horizontal_wrapped(|ui| {
                                            ui.checkbox(&mut step.enabled, "");
                                            ui.label(
                                                egui::RichText::new(format!("{:02}", index + 1))
                                                    .monospace()
                                                    .color(tokens.dim),
                                            );
                                            ui.label(egui::RichText::new(name).strong());
                                            badge(
                                                ui,
                                                self.theme,
                                                &step.last_status,
                                                status_fill(self.theme, &step.last_status),
                                            );
                                        });
                                        ui.label(
                                            egui::RichText::new(detail).small().color(tokens.muted),
                                        );
                                        if !step.output_preview.is_empty() {
                                            ui.label(
                                                egui::RichText::new(&step.output_preview)
                                                    .small()
                                                    .monospace()
                                                    .color(tokens.muted),
                                            );
                                        }
                                        ui.add_space(3.0);
                                        ui.horizontal_wrapped(|ui| {
                                            if ui.small_button("up").clicked() {
                                                move_step = Some((index, -1));
                                            }
                                            if ui.small_button("down").clicked() {
                                                move_step = Some((index, 1));
                                            }
                                            if ui.small_button("copy").clicked() {
                                                duplicate_step = Some(index);
                                            }
                                            if ui.small_button("x").clicked() {
                                                remove_step = Some(index);
                                            }
                                        });
                                    });
                                    ui.add_space(5.0);
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
        });
    }

    fn render_selected_details(&self, ui: &mut egui::Ui, selected_spec: Option<&OperationSpec>) {
        panel_frame(self.theme).show(ui, |ui| {
            section_heading(ui, self.theme, self.language.current_tool(), None);
            if let Some(spec) = selected_spec {
                let tokens = ui_tokens(self.theme);
                ui.label(egui::RichText::new(self.language.operation_name(spec)).strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{} · {} · {}",
                        spec.id, spec.backend, spec.safety
                    ))
                    .small()
                    .color(tokens.muted),
                );
                ui.horizontal_wrapped(|ui| {
                    badge(
                        ui,
                        self.theme,
                        &spec.priority,
                        priority_fill(self.theme, &spec.priority),
                    );
                    badge(
                        ui,
                        self.theme,
                        &spec.safety,
                        safety_fill(self.theme, &spec.safety),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "{}: {}",
                            self.language.output_label(),
                            spec.output.join(" / ")
                        ))
                        .small()
                        .color(tokens.muted),
                    );
                });
                ui.label(
                    egui::RichText::new(category_title(&spec.category, self.language))
                        .small()
                        .color(tokens.dim),
                );
            } else {
                empty_state(
                    ui,
                    self.theme,
                    self.language.no_operation_selected(),
                    text(
                        self.language,
                        "Select a tool from the library.",
                        "从工具库选择一个工具。",
                    ),
                );
            }
        });
    }

    fn render_io_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        panel_frame(self.theme).show(ui, |ui| {
            ui.set_width(ui.available_width().max(320.0));
            ui.horizontal(|ui| {
                section_heading(ui, self.theme, self.language.input_heading(), None);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    for kind in ["file", "bytes", "text"] {
                        ui.selectable_value(&mut self.input_kind, kind.to_string(), kind);
                    }
                });
            });

            if self.input_kind == "file" {
                let editor_width = ui.available_width().max(260.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.file_path)
                        .hint_text("/path/to/file")
                        .desired_width(editor_width),
                );
            } else {
                let editor_width = ui.available_width().max(260.0);
                ui.add(
                    egui::TextEdit::multiline(&mut self.input)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(13)
                        .desired_width(editor_width),
                );
            }

            ui.horizontal_wrapped(|ui| {
                if primary_button(ui, self.theme, self.language.run_recipe()).clicked() {
                    self.run_recipe();
                }
                if soft_button(ui, self.theme, self.language.run_selected()).clicked() {
                    self.run_selected();
                }
                if soft_button(ui, self.theme, self.language.clear_input()).clicked() {
                    if self.input_kind == "file" {
                        self.file_path.clear();
                    } else {
                        self.input.clear();
                    }
                }
                if soft_button(ui, self.theme, self.language.copy_result()).clicked()
                    && !self.output.is_empty()
                {
                    ctx.copy_text(self.output.clone());
                    self.last_status = self.language.copied().to_string();
                }
            });

            if !self.trace.is_empty() {
                ui.collapsing(text(self.language, "Trace", "链路"), |ui| {
                    for item in &self.trace {
                        ui.label(
                            egui::RichText::new(item)
                                .small()
                                .color(ui_tokens(self.theme).muted),
                        );
                    }
                });
            }
            if !self.warnings.is_empty() {
                ui.collapsing(text(self.language, "Warnings", "警告"), |ui| {
                    for warning in &self.warnings {
                        ui.colored_label(ui_tokens(self.theme).warning, warning);
                    }
                });
            }

            ui.separator();
            ui.horizontal(|ui| {
                section_heading(ui, self.theme, self.language.result_heading(), None);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("{} bytes", self.output.len()))
                            .color(ui_tokens(self.theme).muted),
                    );
                });
            });
            let output_width = ui.available_width().max(260.0);
            ui.add(
                egui::TextEdit::multiline(&mut self.output)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(18)
                    .desired_width(output_width),
            );
        });
    }
}

impl eframe::App for CtfToolsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        install_style_for(ctx, self.theme);
        self.handle_global_shortcuts(ctx);
        let total_tools = self
            .registry
            .as_ref()
            .map(|registry| registry.operations().len())
            .unwrap_or_default();

        egui::TopBottomPanel::top("top")
            .exact_height(48.0)
            .show(ctx, |ui| {
                ui.add_space(5.0);
                self.render_top_toolbar(ui, ctx, total_tools);
            });

        if self.workspace == AppWorkspace::Launcher {
            self.render_launcher(ctx);
        } else {
            self.render_operations_workspace(ctx);
        }
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
