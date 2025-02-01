use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::drone::components::*;
use rand::Rng;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::NodeId;
use crate::drone::style::*;
use crate::simulation_controller::{SelectedDrone, SimulationController};

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

//Joins the connected drones with a line
pub fn join(
    mut gizmos: Gizmos,
    drone_query: Query<(&ScDrone, &Transform), With<ScDrone>>,
    simulation_controller: Res<SimulationController>,
) {
    for (drone1, trans1) in drone_query.iter() {
        if !drone1.crashed {
            let start = trans1.translation.truncate(); // Convert to Vec2

            for conn_drone in simulation_controller.drones.iter() {
                if conn_drone.connected_node_ids.contains(&drone1.id) && !get_drone_crashed_form_id(&drone_query, &conn_drone.id){

                    if let Some(trans2) = get_drone_transform_from_id(&drone_query, &conn_drone.id) {
                        let end = trans2.translation.truncate(); // Convert to Vec2

                        // Draw a red line between the two drones
                        gizmos.line_2d(start, end, Color::srgb(1.0, 0.0, 0.0));
                    }
                }
            }
        }
    }
}

// Command -> Spawn = spawns a drone in a random position given his NodeId
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
            texture: asset_server.load("/Volumes/SSD FILIPPO/UNI/Advanced Programming/Drone Project/LeDron-James/src/simulation_controller/assets/sprites/drone.png"),
            transform: Transform::from_xyz(x, y, 1.0),  //for now the drones are spawned in a random position
            ..default()
        },
        ScDrone {
            id: drone.clone(),
            crashed: false,
        },
        //GlobalTransform::default()
    ));
    info!("Spawned drone {} at ({}, {})", drone.clone(), x, y);
    
}

pub fn get_drone_transform_from_id(
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

pub fn get_drone_crashed_form_id(
    drone_query: &Query<(&ScDrone, &Transform), With<ScDrone>>,
    id: &NodeId
) -> bool {
    for (drone, _) in drone_query.iter() {
        if drone.id == *id {
            return drone.crashed
        }
    } false
}

/// Command -> DroneCommand::Crash
pub fn crash(
    simulation_controller: &Res<SimulationController>,
    drone_query: &mut Query<&mut ScDrone ,With<ScDrone>>,
    node_id: &NodeId
) {
    for mut drone in drone_query.iter_mut() {
        if drone.id == *node_id {
            if !drone.crashed {
                drone.crashed = true;
                simulation_controller.drone_command_sender.get(node_id).unwrap().send(DroneCommand::Crash).expect("ERROR crashing drone");
            }
        }
    }


}
/// Command -> DroneCommand::AddSender(NodeId, Sender<Packet>)
pub fn add_sender(
    node_id: &NodeId,
    simulation_controller: &Res<SimulationController>,
    selected_node: &Res<SelectedDrone>
) {
    let current_drone = selected_node.drone_id.unwrap();
    let command_forward = DroneCommand::AddSender(*node_id, simulation_controller.packet_sender.get(node_id).unwrap().clone());
    let command_backwards = DroneCommand::AddSender(current_drone, simulation_controller.packet_sender.get(&current_drone).unwrap().clone());
    //adding the sender to the selected drone
    simulation_controller.drone_command_sender.get(&current_drone).unwrap().send(command_forward).expect("ERROR adding sender forward");
    //adding the selected drone's sender to the new neighbour
    simulation_controller.drone_command_sender.get(&node_id).unwrap().send(command_backwards).expect("ERROR adding sender backwards");
}

/// Command -> DroneCommand::RemoveSender(NodeId)
pub fn remove_sender(
    node_id: &NodeId,
    simulation_controller: &Res<SimulationController>,
    selected_node: &Res<SelectedDrone>
) {
    let current_drone = selected_node.drone_id.unwrap();
    let command_forward = DroneCommand::RemoveSender(*node_id);
    let command_backwards = DroneCommand::RemoveSender(current_drone);
    //adding the sender to the selected drone
    simulation_controller.drone_command_sender.get(&current_drone).unwrap().send(command_forward).expect("ERROR removeing sender forward");
    //adding the selected drone's sender to the new neighbour
    simulation_controller.drone_command_sender.get(&node_id).unwrap().send(command_backwards).expect("ERROR removeing sender backwards");
}

/// Command -> DroneCommand::SetPacketDropRate(f32)
pub fn set_pdr(
    selected_node: &Res<SelectedDrone>,
    simulation_controller: &Res<SimulationController>,
    new_pdr: f32
) {
    let current_node = selected_node.drone_id.unwrap();
    simulation_controller.drone_command_sender.get(&current_node).unwrap().send(DroneCommand::SetPacketDropRate(new_pdr)).expect("ERROR setting pdr");

}

pub fn handle_drone_packet_sent(
    simulation_controller: Res<SimulationController>,
    drone_query: Query<(&ScDrone, &Transform), With<ScDrone>>,
) {
    for reciever in simulation_controller.drone_event_receiver.iter() {
        for recieved in reciever.1.iter() {
            match recieved {
                DroneEvent::PacketSent(packet) => {
                    println!("Packet sent: {:?}", packet);
                },
                DroneEvent::PacketDropped(packet) => {
                    println!("Packet dropped: {:?}", packet);
                },
                DroneEvent::ControllerShortcut(packet) => {
                    println!("Controller shortcut: {:?}", packet);
                }
            }
        }
    }
}



