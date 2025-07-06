/// Tab n.03 data
#[derive(Debug, Clone)]
pub(crate) struct Tab3 {
    pub navigation_level: NavigationLevel, // Current navigation level
    pub selected_outer_box: u8, // 0: Download Folder, 1: Logging Preferences
    pub selected_middle_box: u8, // 0: Choose Target, 1: Choose Log Type (within Logging Preferences)
    pub selected_category: u8, // Index of selected category in right panel

    // Multi-select target system
    pub selected_targets: Vec<String>, // List of selected targets
    pub target_cursor: usize, // Which target we're currently highlighting
    pub available_targets: Vec<String>,

    // Multi-select log level system
    pub selected_log_levels: Vec<LogLevel>, // List of selected log levels
    pub log_level_cursor: usize, // Which log level we're currently highlighting

    pub browsing_for_download_folder: bool, // Flag to know if we're browsing for download folder
}

#[derive(Debug, Clone, PartialEq)]
pub enum NavigationLevel {
    OuterBoxes,      // Selecting between Download Folder and Logging Preferences
    MiddleBoxes,     // Selecting within Logging Preferences (Target/Log Type)
    Categories,      // Selecting categories in right panel
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl Default for Tab3 {
    fn default() -> Self {
        Self {
            navigation_level: NavigationLevel::OuterBoxes,
            selected_outer_box: 0,
            selected_middle_box: 0,
            selected_category: 0,

            // Initialize with "main" target selected
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

            // Initialize with Debug log level selected
            selected_log_levels: vec![LogLevel::Debug],
            log_level_cursor: 1, // Start at Debug (index 1)

            browsing_for_download_folder: false,
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
            LogLevel::Fatal => "FATAL",
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        match self {
            LogLevel::Trace => ratatui::style::Color::Magenta,
            LogLevel::Debug => ratatui::style::Color::Green,
            LogLevel::Info => ratatui::style::Color::White,
            LogLevel::Warn => ratatui::style::Color::Yellow,
            LogLevel::Error => ratatui::style::Color::Red,
            LogLevel::Fatal => ratatui::style::Color::LightRed,
        }
    }

    pub fn all_levels() -> Vec<LogLevel> {
        vec![
            LogLevel::Trace,
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
            LogLevel::Fatal,
        ]
    }
}