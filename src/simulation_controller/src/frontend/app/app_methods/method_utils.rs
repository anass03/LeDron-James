/*
In this module you will find general not-essential methods.
*/
use std::fs::File;
use std::{fs, io};
use std::io::{BufRead, Write};
use std::path::Path;
use ratatui::text::Text;
use tui_textarea::TextArea;
use initializer_utils::utils::{check_config, parse_config};
use crate::frontend::app::ui::tabs::SelectedTab;
use super::super::App;


fn read_from_file<'a>(file: File) -> io::Result<TextArea<'a>> {
    io::BufReader::new(file).lines().collect()
}
impl App<'_> {
    pub(super) fn save_text_buffer(&mut self) {
        let lines = self.cache.text_buffer.text_area.clone().into_lines();
        let mut file = File::create(Path::new(self.settings.toml_path.as_str())).unwrap();
        for x in lines {
            writeln!(file, "{}", x); // adds a newline after each string
        }
    }
    pub(crate) fn reload_file_to_buf(&mut self) {
        match File::open(Path::new(self.settings.toml_path.as_str())) {
            Ok(mut file) => {
                match read_from_file(file) {
                    Ok(text) => {
                        self.cache.text_buffer.text_area = text;
                    }
                    Err(e) => {
                        println!("Error reading file: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("Error opening file: {:?}", e);
                // File doesn't exist
                // 1 - Create file
                // 2 - Link new created file path to settings
                // 3 - Test open, if it doesn't work AGAIN, set app status to failure
            }
        }
    }
    pub(crate) fn next_tab(&mut self) {
        self.cache.cursor = 0;
        self.selected_tab = self.selected_tab.next()
    }
    pub(crate) fn previous_tab(&mut self) {
        self.cache.cursor = 0;
        self.selected_tab = self.selected_tab.prev()
    }
    // Early version of the compiler, just calls parse_config from the ledron git
    // branch and pushes it directly to the compiler
    /*pub(crate) fn compiler_beta(file_path: &str) -> Result<(), String> {
        match parse_config(file_path) {
            Ok(config) => { check_config(&config) }
            Err(e) => {
                Err(e.to_string())
            }
        }
    }*/
}