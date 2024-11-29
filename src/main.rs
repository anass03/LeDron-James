use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize,Debug)]
struct Example {
    id: u32,
    name: String,
}

fn main() {
    let data = Example { id: 42, name: "Test".to_string() };

    // Serializzazione in JSON
    let serialized = serde_json::to_string(&data).unwrap();
    println!("Serialized: {}", serialized);

    // Deserializzazione da JSON
    let deserialized: Example = serde_json::from_str(&serialized).unwrap();
    println!("Deserialized: {:?}", deserialized);
}
