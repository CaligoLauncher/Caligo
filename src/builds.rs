//! UI-only instance preview. No filesystem, network, launch or installed-build state.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum BuildTab {
    #[default]
    Content,
    Files,
    Logs,
    Share,
    Settings,
}

impl BuildTab {
    pub(crate) const TABS: [Self; 4] = [Self::Content, Self::Files, Self::Logs, Self::Share];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Content => "Содержимое",
            Self::Files => "Файлы",
            Self::Logs => "Логи",
            Self::Share => "Поделиться",
            Self::Settings => "Параметры",
        }
    }

    pub(crate) fn empty_title(self) -> &'static str {
        match self {
            Self::Content => "Здесь будет содержимое сборки",
            Self::Files => "Файлы ещё не подключены",
            Self::Logs => "Запусков пока нет",
            Self::Share => "Обмен сборками — позже",
            Self::Settings => "Параметры сборки",
        }
    }

    pub(crate) fn explanation(self) -> &'static str {
        match self {
            Self::Content => "Моды, ресурспаки и шейдеры появятся здесь после подключения управления сборками.",
            Self::Files => "У примера нет игровой папки. Этот экран ничего не читает и не изменяет на диске.",
            Self::Logs => "Minecraft в этой версии не запускается. Здесь нет выдуманных логов или времени игры.",
            Self::Share => "Экспорт и ссылки на сборки ещё не подключены. Никакие данные никуда не отправляются.",
            Self::Settings => "Это параметры примера для оценки интерфейса, не установленная конфигурация игры.",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BuildControl {
    Example,
    Empty,
    Tab(BuildTab),
    TryExample,
}

impl BuildControl {
    pub(crate) const ALL: [Self; 8] = [
        Self::Example, Self::Empty, Self::Tab(BuildTab::Settings),
        Self::Tab(BuildTab::Content), Self::Tab(BuildTab::Files),
        Self::Tab(BuildTab::Logs), Self::Tab(BuildTab::Share), Self::TryExample,
    ];

    pub(crate) fn index(self) -> usize {
        Self::ALL.iter().position(|candidate| *candidate == self).unwrap()
    }

    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Example => "build-example",
            Self::Empty => "build-empty-library",
            Self::Tab(BuildTab::Content) => "build-content",
            Self::Tab(BuildTab::Files) => "build-files",
            Self::Tab(BuildTab::Logs) => "build-logs",
            Self::Tab(BuildTab::Share) => "build-share",
            Self::Tab(BuildTab::Settings) => "build-settings",
            Self::TryExample => "build-try-example",
        }
    }

    pub(crate) fn selected(self, model: &BuildPreview) -> bool {
        match self {
            Self::Example => model.example_visible,
            Self::Empty => !model.example_visible,
            Self::Tab(tab) => model.example_visible && model.tab == tab,
            Self::TryExample => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BuildPreview {
    pub(crate) example_visible: bool,
    pub(crate) tab: BuildTab,
}

impl Default for BuildPreview {
    fn default() -> Self {
        Self { example_visible: true, tab: BuildTab::Content }
    }
}

impl BuildPreview {
    /// Only preview navigation changes. Neither action creates or removes an instance.
    pub(crate) fn apply(&mut self, control: BuildControl) -> bool {
        let previous = *self;
        match control {
            BuildControl::Example | BuildControl::TryExample => {
                self.example_visible = true;
                self.tab = BuildTab::Content;
            }
            BuildControl::Empty => {
                self.example_visible = false;
                self.tab = BuildTab::Content;
            }
            BuildControl::Tab(tab) if self.example_visible => self.tab = tab,
            BuildControl::Tab(_) => {}
        }
        previous != *self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_preview_opens_content() {
        let preview = BuildPreview::default();
        assert!(preview.example_visible);
        assert_eq!(preview.tab, BuildTab::Content);
    }

    #[test]
    fn all_detail_views_are_reachable() {
        let mut preview = BuildPreview::default();
        for tab in [BuildTab::Files, BuildTab::Logs, BuildTab::Share, BuildTab::Settings, BuildTab::Content] {
            assert!(preview.apply(BuildControl::Tab(tab)));
            assert_eq!(preview.tab, tab);
        }
    }

    #[test]
    fn empty_library_clears_detail_selection() {
        let mut preview = BuildPreview::default();
        preview.apply(BuildControl::Tab(BuildTab::Settings));
        assert!(preview.apply(BuildControl::Empty));
        assert!(!preview.example_visible);
        assert_eq!(preview.tab, BuildTab::Content);
        assert!(!preview.apply(BuildControl::Tab(BuildTab::Files)));
    }

    #[test]
    fn example_can_be_reopened_from_both_entry_points() {
        for control in [BuildControl::Example, BuildControl::TryExample] {
            let mut preview = BuildPreview::default();
            preview.apply(BuildControl::Empty);
            assert!(preview.apply(control));
            assert_eq!(preview, BuildPreview::default());
        }
    }

    #[test]
    fn reselecting_does_not_request_a_frame() {
        let mut preview = BuildPreview::default();
        assert!(!preview.apply(BuildControl::Example));
        assert!(!preview.apply(BuildControl::Tab(BuildTab::Content)));
        assert!(preview.apply(BuildControl::Empty));
        assert!(!preview.apply(BuildControl::Empty));
    }

    #[test]
    fn control_ids_and_focus_slots_are_unique() {
        for (i, control) in BuildControl::ALL.iter().enumerate() {
            assert_eq!(control.index(), i);
            for other in &BuildControl::ALL[i + 1..] {
                assert_ne!(control.id(), other.id());
            }
        }
    }

    #[test]
    fn selection_follows_the_visible_view() {
        let mut preview = BuildPreview::default();
        assert!(BuildControl::Example.selected(&preview));
        assert!(!BuildControl::Empty.selected(&preview));
        preview.apply(BuildControl::Tab(BuildTab::Settings));
        assert!(BuildControl::Tab(BuildTab::Settings).selected(&preview));
        assert!(!BuildControl::Tab(BuildTab::Content).selected(&preview));
        preview.apply(BuildControl::Empty);
        assert!(BuildControl::Empty.selected(&preview));
        assert!(!BuildControl::Tab(BuildTab::Settings).selected(&preview));
    }
}