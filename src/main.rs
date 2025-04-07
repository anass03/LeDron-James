pub use drone::Drone;
use simulation_controller::simulation_controller::log;

fn main() {
    let (controller, mut handles) = initializer::initialize();
    log("Sei qui");
    //-------------SIMULATION CONTROLLER----------------
    simulation_controller::simulation_controller::simulate(controller, handles);
    /*while let Some(handle) = handles.pop() {
        log("Closing node thread");
        handle.join().unwrap();
    }
    log("The End")*/
}
