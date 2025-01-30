use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::drone::components::ScDrone;
use crate::drone::systems::*;
use crate::simulation_controller::{SimulationController, SelectedNode};
use crate::ui::option_menu::components::*;
use crate::ui::option_menu::styles::*;
use crate::ui::option_menu::systems::layouts::{spawn_node_id_checkboxes, spawn_set_pdr_button_menu};

pub fn interact_with_crash_button(
    mut button_query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<CrashButton>)>,
    simulation_controller: Res<SimulationController>,
    mut drone_query: Query<&mut ScDrone ,With<ScDrone>>,
    selected_node: Res<SelectedNode>
) {
    if let Ok((interaction, mut background_color)) = button_query.get_single_mut() {
        match *interaction {
            Interaction::Pressed => {
                *background_color = PRESSED_BUTTON_COLOR.into();
                let node_id = match selected_node.node_id {
                    Some(id) => Some(id),
                    None => None
                };
                crash(&simulation_controller, &mut drone_query, &node_id.unwrap());
            }
            Interaction::Hovered => {
                *background_color = HOVERED_BUTTON_COLOR.into();
            }
            Interaction::None => {
                *background_color = NORMAL_BUTTON_COLOR.into();
            }
        }
    }
}

pub fn interact_with_add_sender_button(
    mut commands: Commands,
    mut button_query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<AddSenderButton>)>,
    drone_query: Query<&ScDrone ,With<ScDrone>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut next_state: ResMut<NextState<SenderStatus>>
) {

    if let Ok((interaction, mut background_color)) = button_query.get_single_mut() {
        match *interaction {
            Interaction::Pressed => {
                println!("SenderStatus set to Adding");
                next_state.set(SenderStatus::Adding);
                *background_color = PRESSED_BUTTON_COLOR.into();
                //for each connected drone add this drone form sender
                spawn_node_id_checkboxes(&mut commands, &drone_query, &window_query);
            }
            Interaction::Hovered => {
                *background_color = HOVERED_BUTTON_COLOR.into();
            }
            Interaction::None => {
                *background_color = NORMAL_BUTTON_COLOR.into();
            }
        }
    }
}

pub fn interact_with_select_sender_checkbox(
    mut checkbox_query: Query<(&Interaction, &mut BackgroundColor, &mut Checkbox), Changed<Interaction>>,
) {
    for (interaction, mut bg_color, mut checkbox) in checkbox_query.iter_mut() {
        if *interaction == Interaction::Pressed {
            checkbox.checked = !checkbox.checked; // Toggle selection
            // Change color based on selection state
            if checkbox.checked {
                *bg_color = Color::srgb(0.7,0.7,0.7).into();
            } else {
                *bg_color = Color::srgb(0.5,0.5,0.5).into();
            }
        } else if *interaction == Interaction::Hovered {
            *bg_color = Color::srgb(0.6,0.6,0.6).into();
        } else {
            if !checkbox.checked {
                *bg_color = Color::srgb(0.5,0.5,0.5).into();
            }
        }
    }
}

pub fn interact_with_confirm_add_sender_button(
    mut commands: Commands,
    mut button_query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<ConfirmNodeIdButton>)>,
    checkbox_query: Query<&Checkbox>,
    mut selected_senders: ResMut<SelectedSenders>,
    id_selector: Query<Entity, With<NodeIdSelector>>,
    simulation_controller: Res<SimulationController>,
    selected_node: Res<SelectedNode>
) {
    if let Ok((interaction, mut bg_color)) = button_query.get_single_mut() {
        if *interaction == Interaction::Pressed {
            selected_senders.node_ids = checkbox_query.iter()
                .filter(|checkbox| checkbox.checked)
                .map(|checkbox| checkbox.node_id)
                .collect();

            println!("Adding Senders: {:?}", selected_senders.node_ids);
            for node_id in selected_senders.node_ids.iter() {
                add_sender(node_id, &simulation_controller, &selected_node);
            }
            selected_senders.node_ids.clear();

            *bg_color = Color::srgb(0.6,0.6,0.6).into();

            // Remove the UI after selection
            if let Ok(id_selector_menu) = id_selector.get_single() {
                commands.entity(id_selector_menu).despawn_recursive();
            }
        } else if *interaction == Interaction::Hovered {
            *bg_color = Color::srgb(0.5,0.5,0.5).into();
        } else {
            *bg_color = Color::srgb(0.75,0.75,0.75).into();
        }

    }
}

pub fn interact_with_remove_sender_button(
    mut commands: Commands,
    mut button_query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<RemoveSenderButton>)>,
    drone_query: Query<&ScDrone ,With<ScDrone>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut next_state: ResMut<NextState<SenderStatus>>
) {

    if let Ok((interaction, mut background_color)) = button_query.get_single_mut() {
        match *interaction {
            Interaction::Pressed => {
                println!("SenderStatus set to Removing");
                next_state.set(SenderStatus::Removing);
                *background_color = PRESSED_BUTTON_COLOR.into();
                //for each connected drone add this drone form sender
                spawn_node_id_checkboxes(&mut commands, &drone_query, &window_query);
            }
            Interaction::Hovered => {
                *background_color = HOVERED_BUTTON_COLOR.into();
            }
            Interaction::None => {
                *background_color = NORMAL_BUTTON_COLOR.into();
            }
        }
    }
}

pub fn interact_with_confirm_remove_sender_button(
    mut commands: Commands,
    mut button_query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<ConfirmNodeIdButton>)>,
    checkbox_query: Query<&Checkbox>,
    mut selected_senders: ResMut<SelectedSenders>,
    id_selector: Query<Entity, With<NodeIdSelector>>,
    simulation_controller: Res<SimulationController>,
    selected_node: Res<SelectedNode>
) {
    if let Ok((interaction, mut bg_color)) = button_query.get_single_mut() {
        if *interaction == Interaction::Pressed {
            selected_senders.node_ids = checkbox_query.iter()
                .filter(|checkbox| checkbox.checked)
                .map(|checkbox| checkbox.node_id)
                .collect();

            println!("Removing Senders: {:?}", selected_senders.node_ids);
            for node_id in selected_senders.node_ids.iter() {
                remove_sender(node_id, &simulation_controller, &selected_node);
            }
            selected_senders.node_ids.clear();

            *bg_color = Color::srgb(0.6,0.6,0.6).into();

            // Remove the UI after selection
            if let Ok(id_selector_menu) = id_selector.get_single() {
                commands.entity(id_selector_menu).despawn_recursive();
            }
        } else if *interaction == Interaction::Hovered {
            *bg_color = Color::srgb(0.5,0.5,0.5).into();
        } else {
            *bg_color = Color::srgb(0.75,0.75,0.75).into();
        }
    }
}

pub fn interact_with_set_pdr_button(
    mut commands: Commands,
    mut button_query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<SetPdrButton>)>,
    asset_server: Res<AssetServer>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut is_pdr_menu_open: ResMut<IsPdrMenuOpen>,
) {
    if let Ok((interaction, mut bg_color)) = button_query.get_single_mut() {
        match *interaction {
            Interaction::Pressed => {
                *bg_color = PRESSED_BUTTON_COLOR.into();
                if !is_pdr_menu_open.0 { // Check if the menu DOES NOT exist
                    spawn_set_pdr_button_menu(&mut commands, &asset_server, &window_query);
                    is_pdr_menu_open.0 = true;
                }
            },
            Interaction::Hovered => {
                *bg_color = HOVERED_BUTTON_COLOR.into();
            }
            Interaction::None => {
                *bg_color = NORMAL_BUTTON_COLOR.into();
            }
        }
    }

}

pub fn handle_text_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut InputText, &mut Text)>,
) {
    for (mut input, mut text) in query.iter_mut() {
        // Handle backspace
        if keys.just_pressed(KeyCode::Backspace) {
            input.value.pop();
        }

        // Handle number keys
        for key in [
            KeyCode::Digit0,
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::Digit4,
            KeyCode::Digit5,
            KeyCode::Digit6,
            KeyCode::Digit7,
            KeyCode::Digit8,
            KeyCode::Digit9,
            KeyCode::Period,
        ] {
            if keys.just_pressed(key) {
                match key {
                    KeyCode::Period => input.value.push('.'),
                    KeyCode::Digit0 => input.value.push('0'),
                    KeyCode::Digit1 => input.value.push('1'),
                    KeyCode::Digit2 => input.value.push('2'),
                    KeyCode::Digit3 => input.value.push('3'),
                    KeyCode::Digit4 => input.value.push('4'),
                    KeyCode::Digit5 => input.value.push('5'),
                    KeyCode::Digit6 => input.value.push('6'),
                    KeyCode::Digit7 => input.value.push('7'),
                    KeyCode::Digit8 => input.value.push('8'),
                    KeyCode::Digit9 => input.value.push('9'),
                    _ => {},
                }
            }
        }

        // Update displayed text
        text.sections[0].value = input.value.clone();
    }
}

pub fn interact_with_confirm_set_pdr_button(
    mut commands: Commands,
    mut pdr_value: ResMut<SelectedPdr>,
    mut button_query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<ConfirmPdrButton>)>,
    menu_query: Query<Entity, With<PdrMenu>>,
    mut text_query: Query<&mut InputText, With<Text>>,
    selected_node: Res<SelectedNode>,
    simulation_controller: Res<SimulationController>,
    mut is_pdr_menu_open: ResMut<IsPdrMenuOpen>,
) {
    for (interaction,  mut bg_color) in button_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *bg_color = Color::srgb(0.7, 0.7, 0.7).into();
                if let Ok(menu_entity) = menu_query.get_single() {
                    // Try to fetch the text input
                    if let Ok(mut input) = text_query.get_single_mut() {
                        if let Ok(value) = input.value.parse::<i32>() {
                            if (0..=100).contains(&value) {
                                pdr_value.0 = (value/100) as f32;
                                // Update the PDR value in the simulation
                                set_pdr(&selected_node, &simulation_controller, pdr_value.0);
                                // Despawn menu
                                commands.entity(menu_entity).despawn_recursive();
                                is_pdr_menu_open.0 = false;
                            } else {
                                input.value = "".to_string();
                            }


                        }
                    }
                }
            },
            Interaction::Hovered => {
                *bg_color = Color::srgb(0.6, 0.6, 0.6).into();
            },
            Interaction::None => {
                *bg_color = Color::srgb(0.4, 0.4, 0.4).into();
            }
        }
    }
}


