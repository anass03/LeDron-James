//here we implement the Simulation Controller's commands

use wg_2024::controller::DroneCommand;
use crate::simulation_controller::SimulationController;
use bevy::prelude::*;
use crate::drone::components::ScDrone;

//Simulation Controller to Drone
impl SimulationController {
    pub fn crash_all(&mut self) {
        for (_, sender) in self.sender.iter() {
            sender.send(DroneCommand::Crash).unwrap();
        }
    }
}
