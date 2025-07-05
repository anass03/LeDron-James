# LeDron-James

**LeDron-James** is a drone-based simulation and communication framework developed as a project for the Advanced Programming course 2024/2025. It enables you to:

* **Deploy** a network of drones in configurable topologies
* **Route** packets using source routing, complete with fragmentation and error detection
* **Simulate** failures (packet drops, drone crashes) and dynamic network changes
* **Interact** with content servers (text, media) and chat servers

This solution is useful for studying communication protocols in unreliable networks and evaluating robustness and fault tolerance strategies.

## Key Features

* **Network Discovery Protocol**: flooding-based discovery to reconstruct network topology
* **Source Routing**: explicit path headers for direct packet forwarding
* **Fragmentation/Reassembly**: fixed-size message fragmentation and safe reassembly
* **Packet Drop Simulation**: configurable Packet Drop Rate (PDR) via TOML file
* **Simulation Controller**: monitoring and control interface for crashes, PDR adjustments, and link management
* **Client-Server Protocol**: support for HTTP-like text/media interactions and chat messaging

## Architecture

* **Network Initializer**: reads a user-specified TOML file (default `network_init.toml`, but any filename can be passed as an argument) to create drones, clients, servers, and Crossbeam channels according to the defined topology
* **Drones**: perform packet forwarding, apply PDR, and handle simulated crashes
* **Client/Server**: utilize an Assembler component for serialization, fragmentation, and ACK/NACK handling
* **Simulation Controller**: central thread for issuing commands and receiving events

## Client/Server Specification

The client–server protocol in LeDron-James is built on a custom, HTTP‑like exchange over reliable, source‑routed channels. Key elements:

* **Wire Format**: JSON‐serialized `Request` and `Response` enums, encapsulated in a `Message` with session ID and compressed payload.
* **Handshake**: Clients send a `Hello` message specifying supported content types (text, media, chat). Servers reply with `HelloAck` and assign a session.
* **Requests**:

  * `GetContent(path: String)`: retrieve text or media from content servers.
  * `PostMessage(chat_id: String, payload: String)`: send chat messages to communication servers.
  * `Subscribe(chat_id: String)`: register for incoming chat notifications.
* **Responses**:

  * `Content(data: Vec<u8>, content_type: ContentType)`: delivers text or media.
  * `Ack(id: u64)`: acknowledges receipt of a message fragment.
  * `Nack(id: u64, reason: String)`: signals missing or corrupted fragment.
  * `ChatMessage(from: String, payload: String, timestamp: u64)`: incoming chat frame.
* **Fragmentation**: large payloads are split into fixed‐size frames. Each frame carries its index and total count; servers and clients reassemble only after all fragments are received.
* **Error Handling**: NACKs trigger retransmission of specific fragments. Unresponsive sessions timeout after configurable retries.

## User Interface (Ratatui)

LeDron-James provides a terminal‑based dashboard using the Rust `ratatui` crate for real‑time monitoring and control:

* **Layout**: split into panels for:

  * **Topology View**: ASCII graph of drones and links, updated on topology changes.
  * **Event Log**: scrolling window showing INFO, DEBUG, ERROR events with timestamps.
  * **Control Menu**: interactive list for issuing commands (crash, set\_pdr, add/remove link).
  * **Statistics Panel**: live metrics (packet counts, PDR per drone, active sessions).
* **Navigation**: arrow keys to move between panels, Enter to select actions.
* **Interaction**: popup dialogs for confirmation and parameter input (e.g., setting PDR value).
* **Styling**: uses color highlighting for state (green: healthy drones, red: crashed, yellow: high drop rate).

## Prerequisites

* **Rust** >= 1.68
* **crossbeam** >= 0.8
* **serde**, **serde\_json** for serialization
* **clippy** and **rustfmt** configured for pedantic standards (no warnings)

## Installation

```bash
# Clone the repository
git clone https://github.com/anass03/LeDron-James.git
cd LeDron-James
# Build the project
cargo build --release
```

## Configuration

Create or edit `network_init.toml` to define your desired topology:

```toml
# Example network initialization file
drones = []

[[drone]]
id = "D1"
connected_node_ids = ["D2", "C1"]
pdr = 0.05

[[client]]
id = "C1"
connected_drone_ids = ["D1"]

[[server]]
id = "S1"
connected_drone_ids = ["D2", "D3"]
```

* **drone.pdr**: packet drop probability (0.0 to 1.0)
* Each client connects to up to 2 drones, each server to at least 2 drones

## Running the Simulation

```bash
# Run the simulation
cargo run
```

Use the Simulation Controller commands:

* `crash <drone_id>`: simulate a drone crash
* `set_pdr <drone_id> <value>`: update Packet Drop Rate
* `add_link <from> <to>` / `remove_link <from> <to>`: modify network links

## Client Usage

* **Web Client**: interacts with text/media servers via internal HTTP-like protocol
* **Chat Client**: registers users, sends and receives messages with the communication server

## Testing

```bash
# Run the test suite
cargo test -- --nocapture
```

Ensure that your test `network_init.toml` covers various topologies (chain, star, butterfly, tree)

## Contributing

Contributions are welcome via pull requests. Please follow Rust coding conventions and include tests for any new features.

## License

This project is released under the MIT License. See `LICENSE` for details.
