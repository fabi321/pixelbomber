use std::sync::{
    mpsc::{Receiver, SyncSender, TrySendError},
    Arc,
};

use crate::image_handler::Command;

pub fn get_merger(
    source: Receiver<(Arc<Command>, usize)>,
    sink: SyncSender<Arc<Command>>,
) -> impl FnMut() {
    move || {
        let mut last_idx = 0;
        while let Ok((commands, index)) = source.recv() {
            if index > last_idx {
                last_idx = index;
                if let Err(TrySendError::Disconnected(_)) = sink.try_send(commands) {
                    break;
                }
            }
        }
    }
}
