use initializer::initializer::initialize;
fn main() {
    let (mut controller, handles) = initialize();
    //controller.crash(2);
    for (node_id, handle) in handles {
        handle.join().unwrap();
        println!("Closing thread of node: {:?}", node_id);
    }
}
