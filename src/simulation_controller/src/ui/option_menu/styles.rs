use bevy::asset::AssetServer;
use bevy::color::Color;
use bevy::prelude::*;
use crate::simulation_controller::{log, SelectedDrone};
use crate::ui::option_menu::components::DroneIdText;

pub const NORMAL_BUTTON_COLOR: Color = Color::srgb(0.15, 0.15, 0.15);
pub const HOVERED_BUTTON_COLOR: Color = Color::srgb(0.25, 0.25, 0.25);
pub const PRESSED_BUTTON_COLOR: Color = Color::srgb(0.7, 0.7, 0.7);

pub fn get_option_menu_style(x: f32, y: f32) -> Style {
    let mut style = Style::DEFAULT;
    style.position_type = PositionType::Absolute;
    style.left = Val::Px(x);
    style.top = Val::Px(y);
    style.flex_direction = FlexDirection::Column;
    style.justify_content = JustifyContent::Center;
    style.align_items = AlignItems::Center;
    style.column_gap = Val::Px(6.0);
    style.row_gap = Val::Px(6.0);
    style.width = Val::Px(200.0);
    style.height = Val::Px(270.0);
    style.margin = UiRect::all(Val::Auto);
    style.padding = UiRect::all(Val::Auto);
    style
}

pub const TRANSLUCID_COLOR: Color = Color::srgba(0.096, 0.096, 0.096, 0.3);

pub fn get_title_style(asset_server: &Res<AssetServer>) -> TextStyle {
    TextStyle {
        font: asset_server.load("/Volumes/SSD FILIPPO/UNI/Advanced Programming/Drone Project/LeDron-James/src/simulation_controller/assets/fonts/FiraSans-Bold.ttf"),
        font_size: 30.0,
        color: Color::WHITE
    }
}

pub const BUTTON_STYLE: Style =  {
    let mut style = Style::DEFAULT;
    style.justify_content= JustifyContent::Center;
    style.align_items= AlignItems::Center;
    style.width = Val::Px(130.0);
    style.height = Val::Px(60.0);
    style.margin = UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(0.0), Val::Px(8.0));
    style
};
pub const BUTTON_BACKGROUND_COLOR: Color = Color::srgb(0.15, 0.15, 0.15);

pub fn get_button_text_style(asset_server: &Res<AssetServer>) -> TextStyle {
    TextStyle {
        font: asset_server.load("/Volumes/SSD FILIPPO/UNI/Advanced Programming/Drone Project/LeDron-James/src/simulation_controller/assets/fonts/FiraSans-Bold.ttf"),
        font_size: 20.0,
        color: Color::WHITE
    }
}


pub fn update_node_id_text(
    mut text_query: Query<&mut Text, With<DroneIdText>>,
    node_id: Res<SelectedDrone>
) {
    for mut text in text_query.iter_mut() {
        text.sections[0].value = format!("Drone: {}", node_id.drone_id.unwrap());
    }
}
