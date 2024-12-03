use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::drone::components::*;
use rand::Rng;
use wg_2024::network::NodeId;
use crate::drone::style::*;
use crate::client::components::Client;
use crate::simulation_controller::SimulationController;

pub fn spawn(
    commands: &mut Commands,
    window_query: &Query<&Window, With<PrimaryWindow>>,
    asset_server: &Res<AssetServer>,
    drone: &NodeId
) {
    let window = window_query.get_single().unwrap();
    let mut rng = rand::thread_rng();
    let x = rng.gen_range(get_drone_area_x(window));    //keeping the drones spawning spot in a center area
    let y = rng.gen_range(get_drone_area_y(window));    //------------------------------------------------|
    commands.spawn((
        Text2dBundle {
            text: Text {
                sections: vec![TextSection::new(drone.to_string(), default())], //each drone is represented by its NodeId number
                ..default()
            },
            transform: Transform::from_xyz(x, y, 0.0),  //for now the drones are spawned in a random position
                ..default()
        },
        Drone {},
        Interaction::None
    ));
    
}

pub fn spawn_drone(     //this bevy system spawns each drone that has been inserted in the SimContr
    mut commands: Commands,
    window_query: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
    drones: Res<SimulationController>
) {
    for drone in drones.drones.iter() {
        spawn(&mut commands, &window_query, &asset_server, &drone.0);
    }
}

pub fn join(
    mut commands: Commands,
    drone_query: Query<(&Transform, Entity), With<Drone>>,
    client_query: Query<(&Transform, Entity), With<Client>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>
) {
    for (trans_drone, entity) in drone_query.iter() {
        let start = trans_drone.translation;
        for (trans_client, client) in client_query.iter() {
            let end = trans_client.translation;
            let mid_point = (start + end) / 2.0;
            let length = start.distance(end);
            let angle = (end.y - start.y).atan2(end.x - start.x);

            commands.spawn( SpriteBundle {
                sprite: Sprite {
                    color: Color::srgb(1.0,0.0,0.0),
                    custom_size: Some(Vec2::new(length, 2.0)),
                    ..default()
                },
                transform: Transform {
                    translation: mid_point,
                    rotation: Quat::from_rotation_z(angle),
                    ..default()
                },
                ..default()
            });

        }
    }
}

