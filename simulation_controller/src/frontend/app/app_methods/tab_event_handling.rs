/*
In this module we will handle the specific events related to single tabs.
Our event handler from main App file, will delegate these methods those said events.
 */
use super::super::App;

impl App<'_> {

    fn tab1_send_output(&mut self) {
        // If for any reason parsing doesn't go through, it will just print the error on log
        // Now we have to understand which fn is currently selected
    }


}
pub(super) enum SelectedNode { // Type of node selected, plus function id.
    Drone(u8), Client(u8), Server(u8),
}
