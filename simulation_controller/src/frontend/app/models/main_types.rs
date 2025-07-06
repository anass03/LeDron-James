use std::sync::mpsc::{Receiver, Sender};
use client::Client;
use host::{ClientId, MediaId, ServerId};
use image::math::Rect;
use log::LevelFilter;
use ratatui::Frame;
use ratatui_explorer::{FileExplorer, Theme};
use ratatui_image::thread::ResizeRequest;
use tui_logger::{TuiLoggerSmartWidget, TuiWidgetState};
use wg_2024::network::NodeId;
use crate::backend::sc_models::{FileRendererUpdate, LogType};
use crate::frontend::app::models::tab2_types::Tab2;
use crate::frontend::app::models::general_types::TextBuffer;
use crate::frontend::app::models::tab0_types::Tab0;
use crate::frontend::app::models::tab1_types::{ChThirdPage, Tab1};

pub(in crate::frontend::app) struct Settings{
    pub toml_path: String,
}
impl Settings{
    pub(in crate::frontend::app) fn load() ->Self{
        Self{
            toml_path: "topologies/personalized_network.toml".to_string()
        }
    }
}

#[derive(PartialEq)]
pub(in crate::frontend::app) enum AppMode{
    Normal,
    Input(InputMode),  // u8 represents which input mode I selected:
                // 0 Chat/TOML, 1 Text Buffer first field
    Scrolling,
    FileExploring
}
#[derive(PartialEq)]
pub(crate) enum InputMode{
    Zero, One, Two(NodeId, ChThirdPage)
}



pub(in crate::frontend::app) struct Cache<'a> {
    pub cursor: u8, // Cursor to know which menu on-tab is selected.
    pub colors_vec: (usize, Vec<(u8, u8, u8)>), // List of RGB codes colors fading from Yellow to Purple
    pub counter: u8, // Counting cycles, used to update colors in TUI
    pub main_log_state: (bool, TuiWidgetState),

    pub text_buffer: TextBuffer<'a>,
    pub file_explorer: FileExplorer,

    pub tab0_data: Tab0, // Data needed by tab0 encapsulated in a struct
    pub tab1_data: Tab1,
    pub tab2_data: Tab2
}



impl<'a> Cache<'a> {
    pub(in crate::frontend::app) fn default() -> Cache<'a> {
        Cache {
            text_buffer:TextBuffer::default(),
            cursor:0,
            file_explorer: FileExplorer::new().unwrap(),
            tab0_data: Tab0::default(),
            tab1_data: Tab1::default(),
            tab2_data: Tab2::default(),
            colors_vec: {
                let colors = vec![
                    (179, 250, 178), // #b3fab2
                    (192, 248, 172), // #c0f8ac
                    (204, 245, 168), // #ccf5a8
                    (216, 243, 165), // #d8f3a5
                    (227, 240, 164), // #e3f0a4
                    (237, 238, 164), // #edeea4
                    (246, 235, 165), // #f6eba5
                    (254, 232, 168), // #fee8a8
                    (255, 230, 171), // #ffe6ab
                    (255, 227, 176), // #ffe3b0
                    (255, 227, 187), // #ffe3bb
                    (255, 226, 197), // #ffe2c5
                    (255, 226, 205), // #ffe2cd
                    (255, 226, 211), // #ffe2d3
                    (255, 226, 217), // #ffe2d9
                    (255, 226, 221), // #ffe2dd
                    (255, 226, 226), // #ffe2e2
                    (255, 226, 230), // #ffe2e6
                    (255, 226, 234), // #ffe2ea
                    (255, 226, 238), // #ffe2ee
                    (255, 225, 243), // #ffe1f3
                    (255, 226, 247), // #ffe2f7
                    (255, 228, 250), // #ffe4fa
                    (255, 229, 252), // #ffe5fc
                    (253, 231, 254), // #fde7fe
                    (252, 224, 253), // #fce0fd
                    (251, 218, 253), // #fbdafd
                    (250, 211, 252), // #fad3fc
                    (249, 204, 251), // #f9ccfb
                    (248, 197, 250), // #f8c5fa
                    (247, 190, 249), // #f7bef9
                    (246, 183, 248), // #f6b7f8
                    (245, 176, 247), // #f5b0f7
                    (244, 169, 246), // #f4a9f6
                    (243, 162, 245), // #f3a2f5
                    (241, 155, 244), // #f19bf4
                    (240, 148, 243), // #f094f3
                    (239, 140, 242), // #ef8cf2
                    (238, 132, 240), // #ee84f0
                    (236, 125, 239), // #ec7def
                    (235, 117, 238), // #eb75ee
                    (233, 108, 236), // #e96cec
                    (232,  99, 235), // #e863eb
                    (230,  90, 233), // #e65ae9
                    (229,  80, 232), // #e550e8
                    (227,  69, 230), // #e345e6
                    (225,  56, 228), // #e138e4
                    (224,  40, 227), // #e028e3
                    (222,  14, 225), // #de0ee1
                    (224,  40, 227), // #e028e3
                    (225,  56, 228), // #e138e4
                    (227,  69, 230), // #e345e6
                    (229,  80, 232), // #e550e8
                    (230,  90, 233), // #e65ae9
                    (232,  99, 235), // #e863eb
                    (233, 108, 236), // #e96cec
                    (235, 117, 238), // #eb75ee
                    (236, 125, 239), // #ec7def
                    (238, 132, 240), // #ee84f0
                    (239, 140, 242), // #ef8cf2
                    (240, 148, 243), // #f094f3
                    (241, 155, 244), // #f19bf4
                    (243, 162, 245), // #f3a2f5
                    (244, 169, 246), // #f4a9f6
                    (245, 176, 247), // #f5b0f7
                    (246, 183, 248), // #f6b7f8
                    (247, 190, 249), // #f7bef9
                    (248, 197, 250), // #f8c5fa
                    (249, 204, 251), // #f9ccfb
                    (250, 211, 252), // #fad3fc
                    (251, 218, 253), // #fbdafd
                    (252, 224, 253), // #fce0fd
                    (253, 231, 254), // #fde7fe
                    (253, 231, 254), // #fde7fe
                    (255, 229, 252), // #ffe5fc
                    (255, 228, 250), // #ffe4fa
                    (255, 226, 247), // #ffe2f7
                    (255, 225, 243), // #ffe1f3
                    (255, 226, 238), // #ffe2ee
                    (255, 226, 234), // #ffe2ea
                    (255, 226, 230), // #ffe2e6
                    (255, 226, 226), // #ffe2e2
                    (255, 226, 221), // #ffe2dd
                    (255, 226, 217), // #ffe2d9
                    (255, 226, 211), // #ffe2d3
                    (255, 226, 205), // #ffe2cd
                    (255, 226, 197), // #ffe2c5
                    (255, 227, 187), // #ffe3bb
                    (255, 227, 176), // #ffe3b0
                    (255, 230, 171), // #ffe6ab
                    (254, 232, 168), // #fee8a8
                    (246, 235, 165), // #f6eba5
                    (237, 238, 164), // #edeea4
                    (227, 240, 164), // #e3f0a4
                    (216, 243, 165), // #d8f3a5
                    (204, 245, 168), // #ccf5a8
                    (192, 248, 172), // #c0f8ac
                    (179, 250, 178), // #b3fab2
                ];
                (colors.len(),colors)
            },
            counter: 0,
            main_log_state: (false,TuiWidgetState::default()
                .set_default_display_level(LevelFilter::Off)
                .set_level_for_target("main", LevelFilter::max())),
        }
    }
    pub(in crate::frontend::app) fn update_counter(&mut self) {
        self.counter = self.counter.wrapping_add(1);
    }

}