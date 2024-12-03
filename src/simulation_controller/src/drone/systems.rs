use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::drone::components::*;
use rand::Rng;
use wg_2024::network::NodeId;
use crate::drone::style::*;
use crate::simulation_controller::SimulationController;

pub fn spawn_drone(     //this bevy system spawns each drone that has been inserted in the SimContr
    mut commands: Commands,
    window_query: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
    drones: Res<SimulationController>
) {
    for drone in drones.drones.iter() {
        spawn(&mut commands, &window_query,&asset_server, &drone.id);
    }
}

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
        SpriteBundle {
            texture: asset_server.load("/Users/bibo/UNI/Advanced Programming/Drone Project/LeDron-James/src/simulation_controller/assets/sprites/drone.png"),
            transform: Transform::from_xyz(x, y, 0.0),  //for now the drones are spawned in a random position
            ..default()
        },
        ScDrone { id: drone.clone() },
        Interaction::None
    ));
    
}

pub fn get_drone_from_id(
    drone_query: &Query<(&ScDrone, &Transform), With<ScDrone>>,
    id: &NodeId
) -> Option<Transform> {
    for (drone, transform) in drone_query.iter() {
        if drone.id == *id {
            return Some(transform.clone());
        }
        }
    None
}

pub fn join(                        //this system joins the drones together
    mut commands: Commands,
    drone_query: Query<(&ScDrone, &Transform), With<ScDrone>>,
    simulation_controller: Res<SimulationController>,
) {
    for (drone1, trans1,) in drone_query.iter() {   //iters on the Drone components
        let start = trans1.translation;
        for conn_drone in simulation_controller.drones.iter() {  //iters on the giver connected nodes in the declaration
            if conn_drone.connected_node_ids.contains(&drone1.id) {      //connects two drones if needed
                let end = get_drone_from_id(&drone_query, &conn_drone.id).unwrap().translation;
                let mid_point = (start + end) / 2.0;
                let length = start.distance(end);
                let angle = (end.y - start.y).atan2(end.x - start.x);

                commands.spawn( SpriteBundle {                      //spawns the line that connects two drones
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
}

