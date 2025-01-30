use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use wg_2024::network::NodeId;
use crate::drone::components::ScDrone;
use crate::simulation_controller::SelectedNode;
use crate::ui::option_menu::components::*;
use crate::ui::option_menu::styles::*;

pub fn spawn_option_menu(
    mut commands: Commands,
    window_query: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
    drone_query: Query<(&ScDrone, &Transform), With<ScDrone>>,
    node_id: Res<SelectedNode>,
) {
    build_option_menu(&mut commands, &window_query, &asset_server, &drone_query, &node_id);
}

pub fn despawn_pause_menu(
    mut commands: Commands,
    pause_menu_query: Query<Entity, With<OptionMenu>>
) {
    if let Ok(pause_menu_entity) = pause_menu_query.get_single() {
        commands.entity(pause_menu_entity).despawn_recursive();
    }
}

pub fn build_option_menu(
    commands: &mut Commands,
    window_query: &Query<&Window, With<PrimaryWindow>>,
    asset_server: &Res<AssetServer>,
    drone_query: &Query<(&ScDrone, &Transform), With<ScDrone>>,
    node_id: &Res<SelectedNode>

) -> Entity {
    let mut x = 0.0;
    let mut y = 0.0;
    let window = window_query.get_single().unwrap();
    for drone in drone_query.iter() {
        if drone.0.id == node_id.node_id.unwrap() {
            x = drone.1.translation.x;
            y = window.height() - drone.1.translation.y;
        }
    }

    let option_menu_entity = commands.spawn((
        NodeBundle {
            style: get_option_menu_style(x, y),
            background_color: TRANSLUCID_COLOR.into(),
            ..default()
        },
        OptionMenu {}
    )).with_children(|parent| {
        parent.spawn((
            //-------Drone Id-------
            TextBundle {
                text: Text {
                    sections: vec![
                        TextSection::new(
                            "Drone",
                            get_title_style(&asset_server)
                        )],
                    justify: JustifyText::Center,
                    ..default()
                },
                ..default()
            },
            NodeIdText{}
        ));
        //-------Add Sender Button---------
        parent.spawn((
            ButtonBundle {
                style: BUTTON_STYLE,
                background_color: BUTTON_BACKGROUND_COLOR.into(),
                ..default()
            },
            AddSenderButton {}
        )).with_children(|parent| {
            parent.spawn(
                TextBundle {
                    text: Text {
                        sections: vec![TextSection::new(
                            "ADD SENDER",
                            get_button_text_style(&asset_server)
                        )],
                        ..default()
                    },
                    ..default()

                }
            );
        });
        //-------Remove Sender Button---------
        parent.spawn((
            ButtonBundle {
                style: BUTTON_STYLE,
                background_color: BUTTON_BACKGROUND_COLOR.into(),
                ..default()
            },
            RemoveSenderButton {}
        )).with_children(|parent| {
            parent.spawn(
                TextBundle {
                    text: Text {
                        sections: vec![TextSection::new(
                            "REMOVE SENDER",
                            get_button_text_style(&asset_server)
                        )],
                        ..default()
                    },
                    ..default()

                }
            );
        });
        //-------Set Pdr Button---------
        parent.spawn((
            ButtonBundle {
                style: BUTTON_STYLE,
                background_color: BUTTON_BACKGROUND_COLOR.into(),
                ..default()
            },
            SetPdrButton {}
        )).with_children(|parent| {
            parent.spawn(
                TextBundle {
                    text: Text {
                        sections: vec![TextSection::new(
                            "SET PDR",
                            get_button_text_style(&asset_server)
                        )],
                        ..default()
                    },
                    ..default()

                }
            );
        });
        //-------Crash Button---------
        parent.spawn((
            ButtonBundle {
                style: BUTTON_STYLE,
                background_color: BUTTON_BACKGROUND_COLOR.into(),
                ..default()
            },
            CrashButton {}
        )).with_children(|parent| {
            parent.spawn(
                TextBundle {
                    text: Text {
                        sections: vec![TextSection::new(
                            "CRASH",
                            get_button_text_style(&asset_server)
                        )],
                        ..default()
                    },
                    ..default()

                }
            );
        });
    }).id();
    option_menu_entity
}

pub fn spawn_node_id_checkboxes(
    commands: &mut Commands,
    drone_query: &Query<&ScDrone, With<ScDrone>>,
    window_query: &Query<&Window, With<PrimaryWindow>>,
) -> Entity {
    let available_drones: Vec<NodeId> = drone_query.iter().map(|drone| drone.id).collect();
    println!("{:?}", available_drones);
    let window = window_query.get_single().unwrap();
    let id_selector = commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Px(250.0),
                height: Val::Auto,
                position_type: PositionType::Absolute,
                left: Val::Px(window.width() / 2.0 - 125.0),
                top: Val::Px(window.height() / 2.0 - 125.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: Color::srgb(0.5,0.5,0.5).into(),
            ..default()
        },
        NodeIdSelector {}, // A marker component
    )).with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            "Select Nodes:",
            TextStyle {
                font_size: 18.0,
                color: Color::BLACK,
                ..default()
            },
        ));

        for node_id in available_drones.iter() {
            println!("{node_id:?}");
            parent.spawn((
                ButtonBundle {
                    style: Style {
                        width: Val::Px(200.0),
                        height: Val::Px(30.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: Color::srgb(0.5, 0.5, 0.5).into(),
                    ..default()
                },
                Checkbox { node_id: *node_id, checked: false }, // Custom component
            ))
                .with_children(|checkbox| {
                    checkbox.spawn(TextBundle::from_section(
                        format!("Node {}", node_id),
                        TextStyle {
                            font_size: 16.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ));
                });
        }

        // Confirm Button
        parent.spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(100.0),
                    height: Val::Px(30.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: Color::srgb(0.75,0.75,0.75).into(),
                ..default()
            },
            ConfirmNodeIdButton {},
        ))
            .with_children(|confirm| {
                confirm.spawn(TextBundle::from_section(
                    "Confirm",
                    TextStyle {
                        font_size: 16.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ));
            });
    })
        .id();
    id_selector
}

pub fn spawn_set_pdr_button_menu(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    window_query: &Query<&Window, With<PrimaryWindow>>
) {
    let window = window_query.get_single().unwrap();
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Px(250.0),
                height: Val::Px(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(window.width() / 2.0 - 125.0),
                top: Val::Px(window.height() / 2.0 - 50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            background_color: Color::srgb(0.5, 0.5, 0.5).into(),
            ..default()
        },
        PdrMenu,
    )).with_children(|parent| {
        parent.spawn(
            TextBundle::from_section(
                "Insert new PDR",
                TextStyle {
                    font: asset_server.load("/Volumes/SSD FILIPPO/UNI/Advanced Programming/Drone Project/LeDron-James/src/simulation_controller/assets/fonts/FiraSans-Bold.ttf"),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            )
        );
        parent.spawn((
            TextBundle::from_section(
                "0->100%",
                TextStyle {
                    font: asset_server.load("/Volumes/SSD FILIPPO/UNI/Advanced Programming/Drone Project/LeDron-James/src/simulation_controller/assets/fonts/FiraSans-Bold.ttf"),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ),
            InputText {
                value: String::new()
            },
        ));

        // Confirm Button
        parent.spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(100.0),
                    height: Val::Px(30.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: Color::srgb(0.4, 0.4, 0.4).into(),
                ..default()
            },
            ConfirmPdrButton, // New component for confirmation
        )).with_children(|button| {
            button.spawn(TextBundle::from_section(
                "Confirm",
                TextStyle {
                    font_size: 16.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
        });
    });
}