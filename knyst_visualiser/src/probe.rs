use std::{sync::{Arc, Mutex, OnceLock}, process::id};

use atomic_float::AtomicF32;
use knyst::{prelude::*, Sample};

static NEW_PROBES: OnceLock<Arc<Mutex<Vec<(NodeId, Arc<AtomicF32>)>>>> = OnceLock::new();
pub fn get_new_probes() -> Vec<(NodeId, Arc<AtomicF32>)> {
    let params = NEW_PROBES.get_or_init(|| Arc::new(Mutex::new(Vec::new())));
    let mut params = params.lock().unwrap();
    let return_params = params.clone();
    params.clear();
    return_params
}

fn register_new_probe(node_id: NodeId, value: Arc<AtomicF32>) {
    let params = NEW_PROBES.get_or_init(|| Arc::new(Mutex::new(Vec::new())));
    let mut params = params.lock().unwrap();
    params.push((node_id, value));
}
pub struct Probe {
    value: Arc<AtomicF32>,
}
#[impl_gen]
impl Probe {
    fn new() -> Self {
        let value = Arc::new(AtomicF32::new(0.));
        Self { value }
    }
    pub fn process(&mut self, input: &[Sample]) -> GenState {
        self.value
            .store(input[0], std::sync::atomic::Ordering::SeqCst);
        GenState::Continue
    }
    pub fn init(&mut self, id: NodeId) {
      register_new_probe(id, self.value.clone());
    }
}