/// Tab n.03 data - Settings Page with Menu Area and Opt Area design
#[derive(Debug, Clone)]
pub(crate) struct Tab3 {
    // Menu area cursor and selection (similar to tab1 popup logic)
    pub menu_cursor: usize,
    pub menu_selection: Tab3MenuSelection,

    // Current viewing state
    pub viewing: ViewedTab3,

    // File exploring state
    pub browsing_for_download_folder: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Tab3MenuSelection {
    MenuArea, // Selecting in the menu area (left side)
    OptArea,  // Selecting in the options area (right side)
}

#[derive(Debug, Clone)]
pub(crate) enum ViewedTab3 {
    Main,                           // Main settings page
    SettingsPage(Tab3SettingsPage), // Specific settings page
}

#[derive(Debug, Clone)]
pub(crate) enum Tab3SettingsPage {
    // Download Settings
    DownloadPage(DownloadSettingsPage),

    // Logging Settings
    LoggingPage(LoggingSettingsPage),

    // Network Settings
    NetworkPage(NetworkSettingsPage),

    // Application Settings
    AppPage(AppSettingsPage),
    About(),
}

// Download Settings Page
#[derive(Debug, Clone)]
pub(crate) struct DownloadSettingsPage {
    pub opt_cursor: usize,
    pub selected_item: DownloadSettingsItems,
    pub scroll_offset: u8,
    pub command_popup: DownloadSettingsCommandsPopup,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) enum DownloadSettingsItems {
    #[default]
    DownloadFolder,
}

#[derive(Debug, Clone, Default)]
pub(crate) enum DownloadSettingsCommandsPopup {
    #[default]
    None,
    SetDownloadFolder(CommandPopup),
}

// Logging Settings Page
#[derive(Debug, Clone)]
pub(crate) struct LoggingSettingsPage {
    pub opt_cursor: usize,
    pub selected_item: LoggingSettingsItems,
    pub scroll_offset: u8,
    pub command_popup: LoggingSettingsCommandsPopup,

    // Multi-select target system
    pub selected_targets: Vec<String>,
    pub target_cursor: usize,
    pub available_targets: Vec<String>,

    // Multi-select log level system
    pub selected_log_levels: Vec<LogLevel>,
    pub log_level_cursor: usize,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) enum LoggingSettingsItems {
    #[default]
    LogTargets,
    LogLevels,
    LogFile,
    LogFormat,
}

#[derive(Debug, Clone, Default)]
pub(crate) enum LoggingSettingsCommandsPopup {
    #[default]
    None,
    SetLogTargets(CommandPopup),
    SetLogLevels(CommandPopup),
    SetLogFile(CommandPopup),
    SetLogFormat(CommandPopup),
}

// Network Settings Page
#[derive(Debug, Clone)]
pub(crate) struct NetworkSettingsPage {
    pub opt_cursor: usize,
    pub selected_item: NetworkSettingsItems,
    pub scroll_offset: u8,
    pub command_popup: NetworkSettingsCommandsPopup,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) enum NetworkSettingsItems {
    #[default]
    SetAllPDR,
}

#[derive(Debug, Clone, Default)]
pub(crate) enum NetworkSettingsCommandsPopup {
    #[default]
    None,
    SetAllPDR(CommandPopup),
}

// Application Settings Page
#[derive(Debug, Clone)]
pub(crate) struct AppSettingsPage {
    pub opt_cursor: usize,
    pub selected_item: AppSettingsItems,
    pub scroll_offset: u8,
    pub command_popup: AppSettingsCommandsPopup,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) enum AppSettingsItems {
    #[default]
    RefreshTime,
    TopologyAnimation,
}

#[derive(Debug, Clone, Default)]
pub(crate) enum AppSettingsCommandsPopup {
    #[default]
    None,
    SetRefreshTime(CommandPopup),
    ToggleTopologyAnimation(CommandPopup),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone)]
pub enum RefreshTimeLevel {
    Medium,     // 50ms
    Fast,       // 40ms
    ReallyFast, // 25ms
}

impl RefreshTimeLevel {
    pub fn to_duration(&self) -> std::time::Duration {
        match self {
            RefreshTimeLevel::Medium => std::time::Duration::from_millis(50),
            RefreshTimeLevel::Fast => std::time::Duration::from_millis(40),
            RefreshTimeLevel::ReallyFast => std::time::Duration::from_millis(25),
        }
    }

    pub fn to_string(&self) -> &'static str {
        match self {
            RefreshTimeLevel::Medium => "MEDIUM",
            RefreshTimeLevel::Fast => "FAST",
            RefreshTimeLevel::ReallyFast => "REALLY-FAST",
        }
    }
}

// Common command popup structure (similar to tab1)
#[derive(Clone, Debug)]
pub(crate) struct CommandPopup {
    pub status: SettingsPopupStatus,
    pub cursor: u8,
}

#[derive(Clone, Debug)]
pub(crate) enum SettingsPopupStatus {
    Idle,
    CorrectlySent,
    Error(String),
}

impl Default for Tab3 {
    fn default() -> Self {
        Self {
            menu_cursor: 0,
            menu_selection: Tab3MenuSelection::MenuArea,
            viewing: ViewedTab3::Main,
            browsing_for_download_folder: false,
        }
    }
}

impl Tab3 {
    pub fn menu_up(&mut self) {
        if self.menu_cursor > 0 {
            self.menu_cursor -= 1;
        }
    }

    pub fn menu_down(&mut self) {
        if self.menu_cursor < 3 {
            // 4 menu items: Download, Logging, Network, App
            self.menu_cursor += 1;
        }
    }

    pub fn opt_up(&mut self) {
        if let ViewedTab3::SettingsPage(page) = &mut self.viewing {
            match page {
                Tab3SettingsPage::DownloadPage(p) => {
                    if p.opt_cursor > 0 {
                        p.opt_cursor -= 1;
                    }
                }
                Tab3SettingsPage::LoggingPage(p) => {
                    if p.opt_cursor > 0 {
                        p.opt_cursor -= 1;
                    }
                }
                Tab3SettingsPage::NetworkPage(p) => {
                    if p.opt_cursor > 0 {
                        p.opt_cursor -= 1;
                    }
                }
                Tab3SettingsPage::AppPage(p) => {
                    if p.opt_cursor > 0 {
                        p.opt_cursor -= 1;
                    }
                }
                _ => (),
            }
        }
    }

    pub fn opt_down(&mut self) {
        if let ViewedTab3::SettingsPage(page) = &mut self.viewing {
            match page {
                Tab3SettingsPage::DownloadPage(p) => {
                    if p.opt_cursor < 0 {
                        // 1 item
                        p.opt_cursor += 1;
                    }
                }
                Tab3SettingsPage::LoggingPage(p) => {
                    if p.opt_cursor < 3 {
                        // 4 items
                        p.opt_cursor += 1;
                    }
                }
                Tab3SettingsPage::NetworkPage(p) => {
                    if p.opt_cursor < 0 {
                        // 1 item
                        p.opt_cursor += 1;
                    }
                }
                Tab3SettingsPage::AppPage(p) => {
                    if p.opt_cursor < 1 {
                        // 2 items
                        p.opt_cursor += 1;
                    }
                }
                _ => (),
            }
        }
    }
}

impl LogLevel {
    pub fn to_string(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        match self {
            LogLevel::Trace => ratatui::style::Color::Rgb(237, 254, 173),
            LogLevel::Debug => ratatui::style::Color::Rgb(201, 255, 246),
            LogLevel::Info => ratatui::style::Color::Rgb(182, 255, 206),
            LogLevel::Warn => ratatui::style::Color::Rgb(253, 215, 170),
            LogLevel::Error => ratatui::style::Color::Rgb(255, 168, 168),
        }
    }

    pub fn all_levels() -> Vec<LogLevel> {
        vec![
            LogLevel::Trace,
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
        ]
    }
}

impl Default for DownloadSettingsPage {
    fn default() -> Self {
        Self {
            opt_cursor: 0,
            selected_item: DownloadSettingsItems::DownloadFolder,
            scroll_offset: 0,
            command_popup: DownloadSettingsCommandsPopup::None,
        }
    }
}

impl Default for LoggingSettingsPage {
    fn default() -> Self {
        Self {
            opt_cursor: 0,
            selected_item: LoggingSettingsItems::LogTargets,
            scroll_offset: 0,
            command_popup: LoggingSettingsCommandsPopup::None,
            selected_targets: vec!["main".to_string()],
            target_cursor: 0,
            available_targets: vec![
                "main".to_string(),
                "simulation_controller".to_string(),
                "renderer".to_string(),
                "network".to_string(),
                "topology".to_string(),
                "All targets".to_string(),
            ],
            selected_log_levels: vec![LogLevel::Debug],
            log_level_cursor: 1,
        }
    }
}

impl Default for NetworkSettingsPage {
    fn default() -> Self {
        Self {
            opt_cursor: 0,
            selected_item: NetworkSettingsItems::default(),
            scroll_offset: 0,
            command_popup: NetworkSettingsCommandsPopup::None,
        }
    }
}

impl Default for AppSettingsPage {
    fn default() -> Self {
        Self {
            opt_cursor: 0,
            selected_item: AppSettingsItems::default(),
            scroll_offset: 0,
            command_popup: AppSettingsCommandsPopup::None,
        }
    }
}
