/*use bevy::app::{App, Plugin, Update};
use bevy::prelude::{in_state, AppExtStates, IntoSystemConfigs, OnEnter, OnExit, Startup};
use crate::simulation_controller::{OptionMenu};
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
            .add_systems(Startup, spawn_client_menu) // ✅ Fixed closing )
            .add_systems(OnEnter(OptionMenu::On), spawn_option_menu) // ✅ Fixed closing )
            .add_systems(Update, (
                interact_with_crash_button,
                update_node_id_text,
                interact_with_add_sender_button,
                interact_with_select_sender_checkbox,
                interact_with_confirm_add_sender_button.run_if(in_state(SenderStatus::Adding)),
                interact_with_remove_sender_button,
                interact_with_confirm_remove_sender_button.run_if(in_state(SenderStatus::Removing)),
                interact_with_set_pdr_button,
                interact_with_confirm_set_pdr_button,
                //interact_with_server_type_button,
                //interact_with_file_list_button,
            )) // ✅ Fixed closing )
            .add_systems(Update, (
                /*interact_with_file_button,
                interact_with_confirm_file_name_button,
                interact_with_media_list_button,
                interact_with_media_button,
                interact_with_registration_to_chat_button,
                interact_with_client_list_button,
                interact_with_message_for_button,
                interact_with_confirm_message_for_button,
                handle_file_name_input,
                handle_media_name_input,
                handle_client_message_for_input,
                handle_message_for_input,*/
                handle_pdr_input,
                update_client_text
                ))
        .add_systems(OnExit(OptionMenu::On), despawn_pause_menu); // ✅ Fixed closing )
    }
}*/