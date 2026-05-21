use super::{CtfToolsApp, Language};
use ctf_launcher::{
    ALL_ID, Category, Environment, EnvironmentRegistry, EnvironmentType, FAVORITES_ID,
    LauncherSettings, LauncherStore, LauncherTool, RECENT_ID, ToolType, category_counts,
    filter_tools, launch_tool, record_recent, scan_all_environments,
};
use eframe::egui;

pub struct LauncherUiState {
    store: LauncherStore,
    tools: Vec<LauncherTool>,
    categories: Vec<Category>,
    environments: EnvironmentRegistry,
    settings: LauncherSettings,
    query: String,
    active_category: String,
    selected_tool: Option<String>,
    selected_env: Option<String>,
    editor: ToolEditorState,
    manual_env: EnvEditorState,
    status: String,
}

impl LauncherUiState {
    pub fn load() -> Self {
        let store = LauncherStore::new();
        let tools = store.load_tools();
        let categories = store.load_categories();
        let environments = store.load_environments();
        let settings = store.load_settings();
        let selected_tool = tools.first().map(|tool| tool.id.clone());
        let editor = selected_tool
            .as_deref()
            .and_then(|id| tools.iter().find(|tool| tool.id == id))
            .map(ToolEditorState::from_tool)
            .unwrap_or_default();

        Self {
            store,
            tools,
            categories,
            environments,
            settings,
            query: String::new(),
            active_category: ALL_ID.to_string(),
            selected_tool,
            selected_env: None,
            editor,
            manual_env: EnvEditorState::default(),
            status: "Ready".to_string(),
        }
    }

    fn visible_tools(&self) -> Vec<LauncherTool> {
        filter_tools(&self.tools, &self.active_category, &self.query)
    }

    fn selected_tool(&self) -> Option<&LauncherTool> {
        self.selected_tool
            .as_deref()
            .and_then(|id| self.tools.iter().find(|tool| tool.id == id))
    }

    fn select_tool(&mut self, tool_id: &str) {
        self.selected_tool = Some(tool_id.to_string());
        if let Some(tool) = self.selected_tool() {
            self.editor = ToolEditorState::from_tool(tool);
        }
    }

    fn start_new_tool(&mut self) {
        self.selected_tool = None;
        self.editor = ToolEditorState {
            category: if is_virtual_category(&self.active_category) {
                String::new()
            } else {
                self.active_category.clone()
            },
            ..ToolEditorState::default()
        };
        self.status = "New tool draft".to_string();
    }

    fn save_editor(&mut self, language: Language) {
        if self.editor.name.trim().is_empty() {
            self.status = text(language, "Name is required", "名称不能为空").to_string();
            return;
        }
        let tool = self.editor.to_tool();
        if let Some(index) = self.tools.iter().position(|item| item.id == tool.id) {
            self.tools[index] = tool.clone();
        } else {
            self.tools.push(tool.clone());
        }
        self.ensure_category_for(&tool.category);
        if let Err(error) = self.store.save_tools(&self.tools) {
            self.status = format!("Save failed: {error}");
            return;
        }
        if let Err(error) = self.store.save_categories(&self.categories) {
            self.status = format!("Save categories failed: {error}");
            return;
        }
        self.selected_tool = Some(tool.id);
        self.status = text(language, "Tool saved", "工具已保存").to_string();
    }

    fn delete_selected(&mut self, language: Language) {
        let Some(tool_id) = self.selected_tool.clone() else {
            return;
        };
        self.tools.retain(|tool| tool.id != tool_id);
        if let Err(error) = self.store.save_tools(&self.tools) {
            self.status = format!("Delete failed: {error}");
            return;
        }
        self.selected_tool = self.visible_tools().first().map(|tool| tool.id.clone());
        if let Some(tool) = self.selected_tool() {
            self.editor = ToolEditorState::from_tool(tool);
        } else {
            self.editor = ToolEditorState::default();
        }
        self.status =
            text(language, "Tool removed from launcher", "工具已从启动器移除").to_string();
    }

    fn toggle_favorite(&mut self, language: Language) {
        let Some(tool_id) = self.selected_tool.clone() else {
            return;
        };
        let Some(tool) = self.tools.iter_mut().find(|tool| tool.id == tool_id) else {
            return;
        };
        tool.favorite = !tool.favorite;
        let is_favorite = tool.favorite;
        self.editor.favorite = is_favorite;
        if let Err(error) = self.store.save_tools(&self.tools) {
            self.status = format!("Favorite update failed: {error}");
            return;
        }
        self.status = if is_favorite {
            text(language, "Added to favorites", "已加入收藏")
        } else {
            text(language, "Removed from favorites", "已取消收藏")
        }
        .to_string();
    }

    fn launch_selected(&mut self, language: Language) {
        let Some(tool) = self.selected_tool().cloned() else {
            self.status = text(language, "No tool selected", "未选择工具").to_string();
            return;
        };
        match launch_tool(&tool, &self.environments) {
            Ok(message) => {
                record_recent(&mut self.tools, &tool.id);
                if let Err(error) = self.store.save_tools(&self.tools) {
                    self.status = format!("{message}; recent save failed: {error}");
                } else {
                    self.status = message;
                }
            }
            Err(error) => {
                self.status = format!("Launch failed: {error}");
            }
        }
    }

    fn copy_selected_path(&mut self, ctx: &egui::Context, language: Language) {
        let Some(path) = self.selected_tool().map(|tool| tool.path.clone()) else {
            return;
        };
        ctx.copy_text(path.clone());
        self.status = format!("{}: {path}", text(language, "Copied path", "已复制路径"));
    }

    fn rescan_environments(&mut self, language: Language) {
        let scanned = scan_all_environments();
        self.environments.merge_scanned(scanned);
        if let Err(error) = self.store.save_environments(&self.environments) {
            self.status = format!("Environment scan save failed: {error}");
            return;
        }
        self.status = format!(
            "{}: {}",
            text(language, "Environments scanned", "环境扫描完成"),
            self.environments.environments.len()
        );
    }

    fn add_manual_environment(&mut self, language: Language) {
        if self.manual_env.name.trim().is_empty() || self.manual_env.path.trim().is_empty() {
            self.status = text(
                language,
                "Environment name and path are required",
                "环境名称和路径不能为空",
            )
            .to_string();
            return;
        }
        self.environments.environments.push(Environment {
            id: new_id("user"),
            name: self.manual_env.name.trim().to_string(),
            env_type: self.manual_env.env_type,
            path: self.manual_env.path.trim().to_string(),
            version: self.manual_env.version.trim().to_string(),
            source: "user".to_string(),
            tags: vec!["user".to_string()],
            javafx: self.manual_env.javafx,
        });
        if let Err(error) = self.store.save_environments(&self.environments) {
            self.status = format!("Environment save failed: {error}");
            return;
        }
        self.manual_env = EnvEditorState::default();
        self.status = text(language, "Environment added", "环境已添加").to_string();
    }

    fn remove_selected_environment(&mut self, language: Language) {
        let Some(env_id) = self.selected_env.clone() else {
            return;
        };
        self.environments
            .environments
            .retain(|env| env.id != env_id);
        if self.environments.defaults.python == env_id {
            self.environments.defaults.python.clear();
        }
        if self.environments.defaults.java == env_id {
            self.environments.defaults.java.clear();
        }
        if let Err(error) = self.store.save_environments(&self.environments) {
            self.status = format!("Environment remove failed: {error}");
            return;
        }
        self.selected_env = None;
        self.status = text(language, "Environment removed", "环境已删除").to_string();
    }

    fn set_selected_environment_default(&mut self, language: Language) {
        let Some(env_id) = self.selected_env.clone() else {
            return;
        };
        let Some(env) = self.environments.find(&env_id) else {
            return;
        };
        match env.env_type {
            EnvironmentType::Java => self.environments.defaults.java = env_id,
            EnvironmentType::Python | EnvironmentType::Venv | EnvironmentType::Conda => {
                self.environments.defaults.python = env_id
            }
        }
        if let Err(error) = self.store.save_environments(&self.environments) {
            self.status = format!("Default environment save failed: {error}");
            return;
        }
        self.status = text(language, "Default environment updated", "默认环境已更新").to_string();
    }

    fn ensure_category_for(&mut self, category: &str) {
        let category = category.trim();
        if category.is_empty() || is_virtual_category(category) {
            return;
        }
        if self.categories.iter().any(|item| item.id == category) {
            return;
        }
        self.categories.push(Category {
            id: category.to_string(),
            name: category.to_string(),
            order: self.categories.len() as i64,
        });
    }
}

#[derive(Debug, Clone)]
struct ToolEditorState {
    id: String,
    name: String,
    tool_type: ToolType,
    path: String,
    args: String,
    category: String,
    env_id: String,
    tags: String,
    description: String,
    favorite: bool,
    last_used: u64,
}

impl Default for ToolEditorState {
    fn default() -> Self {
        Self {
            id: new_id("tool"),
            name: String::new(),
            tool_type: ToolType::Shell,
            path: String::new(),
            args: String::new(),
            category: String::new(),
            env_id: String::new(),
            tags: String::new(),
            description: String::new(),
            favorite: false,
            last_used: 0,
        }
    }
}

impl ToolEditorState {
    fn from_tool(tool: &LauncherTool) -> Self {
        Self {
            id: tool.id.clone(),
            name: tool.name.clone(),
            tool_type: tool.tool_type,
            path: tool.path.clone(),
            args: tool.args.clone(),
            category: tool.category.clone(),
            env_id: tool.env_id.clone().unwrap_or_default(),
            tags: tool.tags.join(", "),
            description: tool.description.clone(),
            favorite: tool.favorite,
            last_used: tool.last_used,
        }
    }

    fn to_tool(&self) -> LauncherTool {
        LauncherTool {
            id: if self.id.trim().is_empty() {
                new_id("tool")
            } else {
                self.id.trim().to_string()
            },
            name: self.name.trim().to_string(),
            tool_type: self.tool_type,
            path: self.path.trim().to_string(),
            category: self.category.trim().to_string(),
            env_id: if self.env_id.trim().is_empty() {
                None
            } else {
                Some(self.env_id.trim().to_string())
            },
            args: self.args.trim().to_string(),
            tags: split_tags(&self.tags),
            description: self.description.trim().to_string(),
            favorite: self.favorite,
            last_used: self.last_used,
        }
    }
}

#[derive(Debug, Clone)]
struct EnvEditorState {
    name: String,
    env_type: EnvironmentType,
    path: String,
    version: String,
    javafx: bool,
}

impl Default for EnvEditorState {
    fn default() -> Self {
        Self {
            name: String::new(),
            env_type: EnvironmentType::Python,
            path: String::new(),
            version: String::new(),
            javafx: false,
        }
    }
}

impl CtfToolsApp {
    pub(super) fn render_launcher(&mut self, ctx: &egui::Context) {
        let language = self.language;
        self.handle_launcher_shortcuts(ctx);

        egui::SidePanel::left("launcher_categories")
            .resizable(false)
            .default_width(220.0)
            .show(ctx, |ui| self.render_launcher_categories(ui, language));

        egui::SidePanel::right("launcher_editor")
            .resizable(true)
            .default_width(390.0)
            .show(ctx, |ui| self.render_launcher_editor(ui, language));

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_launcher_main(ui, ctx, language);
        });

        egui::TopBottomPanel::bottom("launcher_status").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(self.launcher.status.as_str());
                ui.separator();
                ui.label(format!(
                    "{}: {}",
                    text(language, "Data", "数据目录"),
                    self.launcher.store.data_dir().display()
                ));
                ui.separator();
                let python = self
                    .launcher
                    .environments
                    .default_python()
                    .map(|env| env.name.as_str())
                    .unwrap_or_else(|| text(language, "not set", "未设置"));
                let java = self
                    .launcher
                    .environments
                    .default_java()
                    .map(|env| env.name.as_str())
                    .unwrap_or_else(|| text(language, "not set", "未设置"));
                ui.label(format!("Python: {python}"));
                ui.separator();
                ui.label(format!("Java: {java}"));
            });
        });
    }

    fn handle_launcher_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|input| input.modifiers.command && input.key_pressed(egui::Key::N)) {
            self.launcher.start_new_tool();
        }
        if ctx.input(|input| input.modifiers.command && input.key_pressed(egui::Key::E))
            && let Some(tool) = self.launcher.selected_tool().cloned()
        {
            self.launcher.editor = ToolEditorState::from_tool(&tool);
        }
        if ctx.input(|input| input.modifiers.command && input.key_pressed(egui::Key::D)) {
            self.launcher.toggle_favorite(self.language);
        }
        if ctx.input(|input| input.key_pressed(egui::Key::Enter)) {
            self.launcher.launch_selected(self.language);
        }
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.launcher.query.clear();
        }
        if ctx.input(|input| input.key_pressed(egui::Key::Backspace)) {
            self.launcher.delete_selected(self.language);
        }
    }

    fn render_launcher_categories(&mut self, ui: &mut egui::Ui, language: Language) {
        ui.heading(text(language, "Launcher", "工具启动器"));
        ui.add_space(6.0);
        let counts = category_counts(&self.launcher.tools);
        let virtual_categories = [
            (ALL_ID, text(language, "All Tools", "全部工具")),
            (FAVORITES_ID, text(language, "Favorites", "我的收藏")),
            (RECENT_ID, text(language, "Recent", "最近启动")),
        ];
        for (category_id, label) in virtual_categories {
            let count = counts.get(category_id).copied().unwrap_or_default();
            self.launcher_category_button(ui, category_id, label, count, language);
        }
        ui.separator();
        if self.launcher.categories.is_empty() {
            ui.label(
                egui::RichText::new(text(language, "No custom categories", "暂无自定义分类"))
                    .small()
                    .color(egui::Color32::from_rgb(145, 158, 174)),
            );
        } else {
            for category in self.launcher.categories.clone() {
                let count = counts.get(&category.id).copied().unwrap_or_default();
                self.launcher_category_button(ui, &category.id, &category.name, count, language);
            }
        }
    }

    fn launcher_category_button(
        &mut self,
        ui: &mut egui::Ui,
        category_id: &str,
        label: &str,
        count: usize,
        language: Language,
    ) {
        let selected = self.launcher.active_category == category_id;
        let response = ui.selectable_label(
            selected,
            format!("{label}\n{}", self.language.tools_count(count)),
        );
        if response.clicked() {
            self.launcher.active_category = category_id.to_string();
            if let Some(tool) = self.launcher.visible_tools().first() {
                self.launcher.select_tool(&tool.id);
            }
        }
        response.on_hover_text(match category_id {
            ALL_ID => text(language, "Show every launcher entry", "显示全部启动器工具"),
            FAVORITES_ID => text(language, "Pinned tools", "收藏工具"),
            RECENT_ID => text(language, "Recently launched tools", "最近启动工具"),
            _ => text(language, "Custom category", "自定义分类"),
        });
    }

    fn render_launcher_main(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, language: Language) {
        ui.horizontal(|ui| {
            ui.heading(text(language, "Local Tool Launcher", "本地工具启动器"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(text(language, "Rescan Envs", "重新扫描环境"))
                    .clicked()
                {
                    self.launcher.rescan_environments(language);
                }
                if ui.button(text(language, "New", "新增")).clicked() {
                    self.launcher.start_new_tool();
                }
            });
        });
        ui.label(text(
            language,
            "asuTools-compatible local launcher for Python, Java, shell, GUI apps, and URLs.",
            "兼容 asuTools 数据结构的本地启动器，支持 Python、Java、Shell、GUI 应用和 URL。",
        ));
        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.launcher.query).hint_text(text(
                language,
                "Search tools, tags, path...",
                "搜索工具、标签、路径...",
            )),
        );
        ui.separator();

        let visible = self.launcher.visible_tools();
        ui.horizontal_wrapped(|ui| {
            ui.label(format!(
                "{} / {}",
                self.language.tools_count(visible.len()),
                self.language.tools_count(self.launcher.tools.len())
            ));
            if !self.launcher.query.is_empty()
                && ui
                    .button(text(language, "Clear search", "清除搜索"))
                    .clicked()
            {
                self.launcher.query.clear();
            }
            if let Some(tool) = self.launcher.selected_tool().cloned() {
                if ui.button(text(language, "Launch", "启动")).clicked() {
                    self.launcher.launch_selected(language);
                }
                if ui
                    .button(if tool.favorite {
                        text(language, "Unfavorite", "取消收藏")
                    } else {
                        text(language, "Favorite", "收藏")
                    })
                    .clicked()
                {
                    self.launcher.toggle_favorite(language);
                }
                if ui.button(text(language, "Copy Path", "复制路径")).clicked() {
                    self.launcher.copy_selected_path(ctx, language);
                }
            }
        });

        ui.separator();
        if visible.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(140.0);
                ui.label(
                    egui::RichText::new(text(
                        language,
                        "No launcher tools yet. Add one from the editor.",
                        "暂无启动器工具，请从右侧编辑区新增。",
                    ))
                    .color(egui::Color32::from_rgb(145, 158, 174)),
                );
            });
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            let card_width = (ui.available_width() - 12.0).max(260.0);
            for tool in visible {
                let selected = self.launcher.selected_tool.as_deref() == Some(tool.id.as_str());
                let response = egui::Frame::group(ui.style())
                    .inner_margin(egui::Margin::same(10))
                    .fill(if selected {
                        egui::Color32::from_rgb(32, 76, 108)
                    } else {
                        egui::Color32::from_rgb(20, 24, 30)
                    })
                    .show(ui, |ui| {
                        ui.set_width(card_width);
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                let favorite = if tool.favorite { " ★" } else { "" };
                                ui.label(
                                    egui::RichText::new(format!("{}{}", tool.name, favorite))
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} · {} · {}",
                                        tool.tool_type.as_str(),
                                        if tool.category.is_empty() {
                                            text(language, "uncategorized", "未分类")
                                        } else {
                                            tool.category.as_str()
                                        },
                                        tool.path
                                    ))
                                    .small()
                                    .color(egui::Color32::from_rgb(145, 158, 174)),
                                );
                                if !tool.description.is_empty() || !tool.tags.is_empty() {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} {}",
                                            tool.description,
                                            tool.tags.join(", ")
                                        ))
                                        .small()
                                        .color(egui::Color32::from_rgb(170, 180, 190)),
                                    );
                                }
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.small_button(text(language, "Launch", "启动")).clicked()
                                    {
                                        self.launcher.select_tool(&tool.id);
                                        self.launcher.launch_selected(language);
                                    }
                                },
                            );
                        });
                    })
                    .response;

                if response.clicked() {
                    self.launcher.select_tool(&tool.id);
                }
                if response.double_clicked() {
                    self.launcher.select_tool(&tool.id);
                    self.launcher.launch_selected(language);
                }
                response.context_menu(|ui| {
                    if ui.button(text(language, "Launch", "启动")).clicked() {
                        self.launcher.select_tool(&tool.id);
                        self.launcher.launch_selected(language);
                        ui.close();
                    }
                    if ui
                        .button(if tool.favorite {
                            text(language, "Unfavorite", "取消收藏")
                        } else {
                            text(language, "Favorite", "收藏")
                        })
                        .clicked()
                    {
                        self.launcher.select_tool(&tool.id);
                        self.launcher.toggle_favorite(language);
                        ui.close();
                    }
                    if ui.button(text(language, "Edit", "编辑")).clicked() {
                        self.launcher.select_tool(&tool.id);
                        ui.close();
                    }
                    if ui.button(text(language, "Copy Path", "复制路径")).clicked() {
                        self.launcher.select_tool(&tool.id);
                        self.launcher.copy_selected_path(ctx, language);
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(text(language, "Remove", "移除")).clicked() {
                        self.launcher.select_tool(&tool.id);
                        self.launcher.delete_selected(language);
                        ui.close();
                    }
                });
                ui.add_space(6.0);
            }
        });
    }

    fn render_launcher_editor(&mut self, ui: &mut egui::Ui, language: Language) {
        ui.heading(text(language, "Tool Editor", "工具编辑"));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button(text(language, "New", "新增")).clicked() {
                self.launcher.start_new_tool();
            }
            if ui.button(text(language, "Save Tool", "保存工具")).clicked() {
                self.launcher.save_editor(language);
            }
            if self.launcher.selected_tool.is_some()
                && ui.button(text(language, "Remove", "移除")).clicked()
            {
                self.launcher.delete_selected(language);
            }
        });
        ui.separator();

        egui::Grid::new("launcher_tool_editor")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label(text(language, "Name", "名称"));
                ui.text_edit_singleline(&mut self.launcher.editor.name);
                ui.end_row();

                ui.label(text(language, "Type", "类型"));
                egui::ComboBox::from_id_salt("launcher_tool_type")
                    .selected_text(self.launcher.editor.tool_type.as_str())
                    .show_ui(ui, |ui| {
                        for tool_type in [
                            ToolType::Python,
                            ToolType::Java,
                            ToolType::Shell,
                            ToolType::Gui,
                            ToolType::Url,
                        ] {
                            ui.selectable_value(
                                &mut self.launcher.editor.tool_type,
                                tool_type,
                                tool_type.as_str(),
                            );
                        }
                    });
                ui.end_row();

                ui.label(text(language, "Path / URL", "路径 / URL"));
                ui.text_edit_singleline(&mut self.launcher.editor.path);
                ui.end_row();

                ui.label(text(language, "Args", "参数"));
                ui.text_edit_singleline(&mut self.launcher.editor.args);
                ui.end_row();

                ui.label(text(language, "Category", "分类"));
                ui.text_edit_singleline(&mut self.launcher.editor.category);
                ui.end_row();

                ui.label(text(language, "Environment", "环境"));
                self.render_env_combo(ui);
                ui.end_row();

                ui.label(text(language, "Tags", "标签"));
                ui.text_edit_singleline(&mut self.launcher.editor.tags);
                ui.end_row();

                ui.label(text(language, "Description", "描述"));
                ui.text_edit_singleline(&mut self.launcher.editor.description);
                ui.end_row();
            });
        ui.checkbox(
            &mut self.launcher.editor.favorite,
            text(language, "Favorite", "收藏"),
        );

        ui.separator();
        ui.collapsing(text(language, "Environments", "环境"), |ui| {
            self.render_environment_settings(ui, language);
        });
        ui.collapsing(text(language, "About asuTools Port", "关于 asuTools 搬运"), |ui| {
            ui.label(text(
                language,
                "This launcher keeps the asuTools JSON schema: tools.json, categories.json, environments.json, settings.json.",
                "此启动器保持 asuTools JSON 数据结构：tools.json、categories.json、environments.json、settings.json。",
            ));
            ui.label("Source: https://github.com/lsdogXG/asutools · MIT");
            ui.label(format!("Theme setting: {}", self.launcher.settings.theme));
        });
    }

    fn render_env_combo(&mut self, ui: &mut egui::Ui) {
        let language = self.language;
        let selected = if self.launcher.editor.env_id.is_empty() {
            text(language, "Follow default", "跟随默认").to_string()
        } else {
            self.launcher
                .environments
                .find(&self.launcher.editor.env_id)
                .map(|env| env.name.clone())
                .unwrap_or_else(|| self.launcher.editor.env_id.clone())
        };
        egui::ComboBox::from_id_salt("launcher_env_combo")
            .selected_text(selected)
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.launcher.editor.env_id,
                    String::new(),
                    text(language, "Follow default", "跟随默认"),
                );
                let wanted: &[EnvironmentType] = match self.launcher.editor.tool_type {
                    ToolType::Python => &[
                        EnvironmentType::Python,
                        EnvironmentType::Venv,
                        EnvironmentType::Conda,
                    ],
                    ToolType::Java => &[EnvironmentType::Java],
                    _ => &[],
                };
                for env in self
                    .launcher
                    .environments
                    .environments
                    .iter()
                    .filter(|env| wanted.is_empty() || wanted.contains(&env.env_type))
                {
                    let suffix = if env.javafx { " +FX" } else { "" };
                    ui.selectable_value(
                        &mut self.launcher.editor.env_id,
                        env.id.clone(),
                        format!("{}{} · {}", env.name, suffix, env.path),
                    );
                }
            });
    }

    fn render_environment_settings(&mut self, ui: &mut egui::Ui, language: Language) {
        ui.horizontal_wrapped(|ui| {
            if ui.button(text(language, "Rescan", "重新扫描")).clicked() {
                self.launcher.rescan_environments(language);
            }
            if ui
                .button(text(language, "Set Default", "设为默认"))
                .clicked()
            {
                self.launcher.set_selected_environment_default(language);
            }
            if ui.button(text(language, "Remove", "删除")).clicked() {
                self.launcher.remove_selected_environment(language);
            }
        });
        ui.add_space(4.0);
        egui::ScrollArea::vertical()
            .max_height(190.0)
            .show(ui, |ui| {
                for env in self.launcher.environments.environments.clone() {
                    let is_default = match env.env_type {
                        EnvironmentType::Java => self.launcher.environments.defaults.java == env.id,
                        EnvironmentType::Python
                        | EnvironmentType::Venv
                        | EnvironmentType::Conda => {
                            self.launcher.environments.defaults.python == env.id
                        }
                    };
                    let selected = self.launcher.selected_env.as_deref() == Some(env.id.as_str());
                    let default_marker = if is_default { " ★" } else { "" };
                    let response = ui.selectable_label(
                        selected,
                        format!(
                            "[{}{}] {}{}\n{}",
                            env.env_type.as_str(),
                            if env.javafx { "+fx" } else { "" },
                            env.name,
                            default_marker,
                            env.path
                        ),
                    );
                    if response.clicked() {
                        self.launcher.selected_env = Some(env.id);
                    }
                }
            });

        ui.separator();
        ui.label(egui::RichText::new(text(language, "Manual Add", "手动添加")).strong());
        egui::Grid::new("manual_env_editor")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label(text(language, "Name", "名称"));
                ui.text_edit_singleline(&mut self.launcher.manual_env.name);
                ui.end_row();

                ui.label(text(language, "Type", "类型"));
                egui::ComboBox::from_id_salt("manual_env_type")
                    .selected_text(self.launcher.manual_env.env_type.as_str())
                    .show_ui(ui, |ui| {
                        for env_type in [
                            EnvironmentType::Python,
                            EnvironmentType::Venv,
                            EnvironmentType::Conda,
                            EnvironmentType::Java,
                        ] {
                            ui.selectable_value(
                                &mut self.launcher.manual_env.env_type,
                                env_type,
                                env_type.as_str(),
                            );
                        }
                    });
                ui.end_row();

                ui.label(text(language, "Path", "路径"));
                ui.text_edit_singleline(&mut self.launcher.manual_env.path);
                ui.end_row();

                ui.label(text(language, "Version", "版本"));
                ui.text_edit_singleline(&mut self.launcher.manual_env.version);
                ui.end_row();
            });
        ui.checkbox(&mut self.launcher.manual_env.javafx, "JavaFX");
        if ui
            .button(text(language, "Add Environment", "添加环境"))
            .clicked()
        {
            self.launcher.add_manual_environment(language);
        }
    }
}

fn text(language: Language, english: &'static str, chinese: &'static str) -> &'static str {
    match language {
        Language::English => english,
        Language::Chinese => chinese,
    }
}

fn is_virtual_category(category: &str) -> bool {
    matches!(category, ALL_ID | FAVORITES_ID | RECENT_ID)
}

fn split_tags(tags: &str) -> Vec<String> {
    tags.split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn new_id(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{prefix}-{:x}", nanos & 0xffff_ffff)
}
