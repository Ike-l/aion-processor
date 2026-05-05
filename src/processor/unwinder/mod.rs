use std::{sync::mpsc::Sender, thread::panicking};

pub struct Unwinder {
    result_tx: Sender<(bool, String)>,
    thread_label: String
}

impl Unwinder {
    pub fn new(result_tx: Sender<(bool, String)>, thread_label: String) -> Self {
        Self { result_tx, thread_label }
    }
}

impl Drop for Unwinder {
    fn drop(&mut self) {
        let _ = self.result_tx.send((panicking(), self.thread_label.clone()));
    }
}