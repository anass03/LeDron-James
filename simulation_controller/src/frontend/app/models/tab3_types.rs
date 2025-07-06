/// Tab n.03 data
#[derive(Debug, Clone)]
pub(crate) struct Tab3 {
    pub navigation_level: NavigationLevel, // Current navigation level
    pub selected_outer_box: u8, // 0: Download Folder, 1: Logging Preferences
    pub selected_middle_box: u8, // 0: Choose Target, 1: Choose Log Type (within Logging Preferences)
    pub selected_category: u8, // Index of selected category in right panel
    pub current_download_folder: String,
    pub current_log_target: String,
    pub current_log_level: LogLevel,
    pub available_targets: Vec<String>,
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
            current_download_folder: "assets".to_string(),
            current_log_target: "main".to_string(),
            current_log_level: LogLevel::Debug,
            available_targets: vec![
                "main".to_string(),
                "simulation_controller".to_string(),
                "renderer".to_string(),
                "network".to_string(),
                "topology".to_string(),
                "All targets".to_string(),
            ],
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

    pub fn next(&self) -> Self {
        match self {
            LogLevel::Trace => LogLevel::Debug,
            LogLevel::Debug => LogLevel::Info,
            LogLevel::Info => LogLevel::Warn,
            LogLevel::Warn => LogLevel::Error,
            LogLevel::Error => LogLevel::Fatal,
            LogLevel::Fatal => LogLevel::Trace,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            LogLevel::Trace => LogLevel::Fatal,
            LogLevel::Debug => LogLevel::Trace,
            LogLevel::Info => LogLevel::Debug,
            LogLevel::Warn => LogLevel::Info,
            LogLevel::Error => LogLevel::Warn,
            LogLevel::Fatal => LogLevel::Error,
        }
    }
}