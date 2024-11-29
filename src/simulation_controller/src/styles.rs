use bevy::prelude::*;

pub fn get_demo_text_style(asset_server: &Res<AssetServer>) -> TextStyle {
    TextStyle {
        font: asset_server.load("/Users/bibo/UNI/Advanced Programming/Drone Project/LeDron-James/src/simulation_controller/assets/fonts/FiraSans-Bold.ttf"),
        font_size: 64.0,
        color: Color::WHITE
    }
}

pub const DEMO_TEXT_STYLE: Style = {
    let mut style = Style::DEFAULT;
    style.flex_direction = FlexDirection::Column;  //children will be on a column
    style.justify_content = JustifyContent::Center;    //children in the column will alligned in the center
    style.align_items = AlignItems::Center;
    style.column_gap = Val::Px(8.0);           //space between children
    style.row_gap = Val::Px(8.0);              //---------------------|
    style.width = Val::Percent(100.0);     //setting the main Parent Node to be the size of the whole window
    style.height = Val::Percent(100.0);
    style
};