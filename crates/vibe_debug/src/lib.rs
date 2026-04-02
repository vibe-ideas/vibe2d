mod server;
mod protocol;

pub use protocol::{VdpRequest, VdpResponse};
pub use server::VdpServer;

use std::sync::mpsc;

/// Channel pair for communication between VDP server and game loop.
pub struct VdpChannel {
    pub receiver: mpsc::Receiver<VdpRequest>,
    pub sender: mpsc::Sender<VdpResponse>,
}

/// Create a VDP channel pair. Returns (game_side, server_side).
pub fn create_channel() -> (VdpChannel, VdpServerChannel) {
    let (req_tx, req_rx) = mpsc::channel();
    let (resp_tx, resp_rx) = mpsc::channel();
    (
        VdpChannel {
            receiver: req_rx,
            sender: resp_tx,
        },
        VdpServerChannel {
            sender: req_tx,
            receiver: resp_rx,
        },
    )
}

/// Server-side channel endpoints.
pub struct VdpServerChannel {
    pub sender: mpsc::Sender<VdpRequest>,
    pub receiver: mpsc::Receiver<VdpResponse>,
}
