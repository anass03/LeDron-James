use bevy::app::{App, Plugin, Update};
use bevy::prelude::{in_state, AppExtStates, IntoSystemConfigs, OnEnter, OnExit};
use crate::simulation_controller::OptionMenu;
use crate::ui::option_menu::components::*;
use crate::ui::option_menu::styles::update_node_id_text;
use crate::ui::option_menu::systems::interactions::*;
use crate::ui::option_menu::systems::layouts::*;

pub mod components;
mod styles;

mod systems;

pub struct OptionMenuPlugin;

impl Plugin for OptionMenuPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_state(OptionMenu::default())
            .insert_state(SenderStatus::default())
            .insert_resource(SelectedSenders::default())
            .insert_resource(SelectedPdr::default())
            .insert_resource(IsPdrMenuOpen(false))
            .add_systems(OnEnter(OptionMenu::On), spawn_option_menu)
            .add_systems(Update, (
                interact_with_crash_button,
                update_node_id_text,
                interact_with_add_sender_button,
                interact_with_select_sender_checkbox,
                interact_with_confirm_add_sender_button.run_if(in_state(SenderStatus::Adding)),
                interact_with_remove_sender_button,
                interact_with_confirm_remove_sender_button.run_if(in_state(SenderStatus::Removing)),
                interact_with_set_pdr_button,
                interact_with_set_pdr_button,
                handle_text_input,
                interact_with_confirm_set_pdr_button,
                ))
            .add_systems(OnExit(OptionMenu::On), despawn_pause_menu)
        ;
    }
}