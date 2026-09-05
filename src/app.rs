use eframe::egui;

use crate::auth::AuthManager;
use crate::theme::ThemePreset;
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Play,
    Instances,
    Settings,
}

pub struct TerraLauncherApp {
    pub tab: Tab,
    pub theme: ThemePreset,
    pub settings: ui::settings::SettingsState,
    pub auth: AuthManager,
}

impl TerraLauncherApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let theme = ThemePreset::default();
        theme.apply(&cc.egui_ctx);
        Self {
            tab: Tab::Play,
            theme,
            settings: Default::default(),
            auth: Default::default(),
        }
    }
}

impl eframe::App for TerraLauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("nav")
            .resizable(false)
            .exact_width(180.0)
            .show(ctx, |ui| {
                ui.add_space(16.0);
                ui.vertical_centered(|ui| {
                    ui.heading("Terra");
                });
                ui.add_space(16.0);
                ui.selectable_value(&mut self.tab, Tab::Play, "Играть");
                ui.selectable_value(&mut self.tab, Tab::Instances, "Сборки");
                ui.selectable_value(&mut self.tab, Tab::Settings, "Настройки");
            });

        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::Play => ui::play::show(ui, &self.auth),
            Tab::Instances => ui::instances::show(ui),
            Tab::Settings => ui::settings::show(ui, &mut self.settings, &mut self.theme),
        });
    }
}
