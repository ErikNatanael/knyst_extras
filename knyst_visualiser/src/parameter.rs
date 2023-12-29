use std::sync::{Arc, Mutex, OnceLock};

use atomic_float::AtomicF32;
use knyst::{prelude::*, Sample};

static NEW_PARAMETERS: OnceLock<Arc<Mutex<Vec<(String, Arc<AtomicF32>)>>>> = OnceLock::new();
pub fn get_new_parameters() -> Vec<(String, Arc<AtomicF32>)> {
    let params = NEW_PARAMETERS.get_or_init(|| Arc::new(Mutex::new(Vec::new())));
    let mut params = params.lock().unwrap();
    let return_params = params.clone();
    params.clear();
    return_params
}

fn register_new_parameter(name: String, value: Arc<AtomicF32>) {
    let params = NEW_PARAMETERS.get_or_init(|| Arc::new(Mutex::new(Vec::new())));
    let mut params = params.lock().unwrap();
    params.push((name, value));
}
pub struct Parameter {
    value: Arc<AtomicF32>,
}
#[impl_gen]
impl Parameter {
    pub fn new(name: impl Into<String>, start: Sample) -> Self {
        let value = Arc::new(AtomicF32::new(start as f32));
        register_new_parameter(name.into(), value.clone());
        Self { value }
    }
    pub fn process(&mut self, output: &mut [Sample]) -> GenState {
        output.fill(self.value.load(std::sync::atomic::Ordering::SeqCst));
        GenState::Continue
    }
}

// pub struct Parameter {
//   value: Arc<AtomicF32>
// }
// impl Parameter {

// }
