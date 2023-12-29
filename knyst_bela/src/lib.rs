use std::sync::mpsc::{channel, Receiver};

use knyst::{
    audio_backend::AudioBackend,
    controller::Controller,
    graph::RunGraph,
    prelude::{Graph,  KnystCommands, RunGraphSettings},
    sphere::{KnystSphere, SphereSettings},
    KnystError, Resources,
};

#[derive(thiserror::Error, Debug)]
pub enum BelaKnystError {
    #[error("Tried to create a BelaKnystStarted from a BelaKnystBackend, but the backend wasn't started")]
    BelaKnystBackendNotStarted,
}
pub struct BelaKnystBackend {
    block_size: usize,
    sample_rate: f32,
    num_inputs: usize,
    num_outputs: usize,
    run_graph: Option<RunGraph>,
    controller: Option<Controller>,
    unhandled_errors: Option<Receiver<KnystError>>,
}

impl AudioBackend for BelaKnystBackend {
    fn start_processing_return_controller(
        &mut self,
        mut graph: Graph,
        resources: Resources,
        run_graph_settings: RunGraphSettings,
        error_handler: Box<dyn FnMut(KnystError) + Send + 'static>,
    ) -> Result<Controller, knyst::audio_backend::AudioBackendError> {
        let _ = error_handler;
        let (run_graph, resources_command_sender, resources_command_receiver) =
            RunGraph::new(&mut graph, resources, run_graph_settings).expect("Since we are creating the Graph here there should be no reason for RunGraph::new to fail");
        self.run_graph = Some(run_graph);
        let (error_producer, error_consumer) = channel();
        self.unhandled_errors = Some(error_consumer);
        let error_handler = move |err| {
            error_producer.send(err).expect("The Receiver should be dropped after anything that can send errors via this callback");
        };
        let controller = Controller::new(
            graph,
            error_handler,
            resources_command_sender,
            resources_command_receiver,
        );
        Ok(controller)
    }

    fn stop(&mut self) -> Result<(), knyst::audio_backend::AudioBackendError> {
        todo!()
    }

    fn sample_rate(&self) -> usize {
        self.sample_rate as usize
    }

    fn block_size(&self) -> Option<usize> {
        Some(self.block_size)
    }

    fn native_output_channels(&self) -> Option<usize> {
        Some(self.num_outputs)
    }

    fn native_input_channels(&self) -> Option<usize> {
        Some(self.num_inputs)
    }
}

impl BelaKnystBackend {
    /// Creates a new BelaKnystBackend as well as a top level graph
    pub fn new(sample_rate: f32, block_size: usize, num_inputs: usize, num_outputs: usize) -> Self {
        Self {
            run_graph: None,
            controller: None,
            unhandled_errors: None,
            block_size,
            sample_rate,
            num_inputs,
            num_outputs,
        }
    }
    pub fn to_started(self) -> Result<BelaKnystStarted, BelaKnystError> {
        Ok(BelaKnystStarted {
            run_graph: self
                .run_graph
                .ok_or(BelaKnystError::BelaKnystBackendNotStarted)?,
            controller: self
                .controller
                .ok_or(BelaKnystError::BelaKnystBackendNotStarted)?,
            unhandled_errors: self
                .unhandled_errors
                .ok_or(BelaKnystError::BelaKnystBackendNotStarted)?,
        })
    }
}

/// A started BelaKnyst implementation.
pub struct BelaKnystStarted {
    run_graph: RunGraph,
    controller: Controller,
    unhandled_errors: Receiver<KnystError>,
}
impl BelaKnystStarted {
    /// Get a KnystCommands. This way, by passing a pointer to the
    /// BelaKnystBackend to your Rust sounds synthesis setup code, that code can
    /// get access to a KnystCommands.
    pub fn knyst_commands(&mut self) -> impl KnystCommands {
        self.controller.get_knyst_commands()
    }
    /// Run updates for Controller and RunGraph. Non realtime thread safe
    pub fn update(&mut self) {
        self.controller.run(500);
    }
    /// Sets the input of one channel to the graph. Run before `process_block`
    pub fn set_input_channel(&mut self, channel_index: usize, input_channel: &[f32]) {
        let graph_input_buffers = self.run_graph.graph_input_buffers();
        assert_eq!(graph_input_buffers.block_size(), input_channel.len());
        for i in 0..graph_input_buffers.block_size() {
            graph_input_buffers.write(input_channel[i], channel_index, i);
        }
    }
    pub fn process_block(&mut self) {
        self.run_graph.run_resources_communication(50);
        self.run_graph.process_block();
    }
    /// Gets the output of one channel from the graph. Run after `process_block`
    pub fn get_output_channel(&self, index: usize) -> &[f32] {
        self.run_graph.graph_output_buffers().get_channel(index)
    }
    pub fn next_error(&mut self) -> Option<KnystError> {
        self.unhandled_errors.try_recv().ok()
    }
}

/// Creates an opaque pointer to the BelaKnystBackend
///
/// # Safety
/// Make sure you destroy the SynthesisInterface with `bela_knyst_backend_destroy`
/// once you are done with it.
#[no_mangle]
pub unsafe extern "C" fn bela_knyst_create(
    sample_rate: libc::c_float,
    block_size: libc::size_t,
    num_inputs: libc::size_t,
    num_outputs: libc::size_t,
) -> *mut BelaKnystStarted {
    let mut backend =
        BelaKnystBackend::new(sample_rate as f32, block_size, num_inputs, num_outputs);
    let (_sphere_id, controller) =
        KnystSphere::start_return_controller(&mut backend, SphereSettings::default(), |_| ())
            .expect("Unable to start KnystSphere using KnystBelaBackend");
    backend.controller = Some(controller);
    let started = backend.to_started().unwrap();
    Box::into_raw(Box::new(started))
}
/// Run updates (non realtime thread safe)
///
/// # Safety
/// Only call with a pointer received from `bela_knyst_backend_create`
#[no_mangle]
pub unsafe extern "C" fn bela_knyst_update(bela_knyst_ptr: *mut BelaKnystStarted) {
    if !bela_knyst_ptr.is_null() {
        (*bela_knyst_ptr).update();
    }
}
/// Set an input channel. Run before process_block
///
/// # Safety
/// Only call with a pointer received from `bela_knyst_backend_create`
#[no_mangle]
pub unsafe extern "C" fn bela_knyst_set_input_channel(
    bela_knyst_ptr: *mut BelaKnystStarted,
    channel_index: libc::size_t,
    input_channel_ptr: *const libc::c_float,
    block_size: libc::size_t,
) {
    if !bela_knyst_ptr.is_null() {
        let input_channel = std::slice::from_raw_parts(input_channel_ptr as *const f32, block_size);
        (*bela_knyst_ptr).set_input_channel(channel_index, input_channel);
    }
}
/// Process one block of audio
///
/// # Safety
/// Only call with a pointer received from `bela_knyst_backend_create`
#[no_mangle]
pub unsafe extern "C" fn bela_knyst_process_block(bela_knyst_backend_ptr: *mut BelaKnystStarted) {
    if !bela_knyst_backend_ptr.is_null() {
        (*bela_knyst_backend_ptr).process_block();
    }
}
/// Get an output channel. Run after process_block
///
/// # Safety
/// Only call with a pointer received from `bela_knyst_backend_create`. Use immediately after receiving it and then never use the pointer again.
#[no_mangle]
pub unsafe extern "C" fn bela_knyst_get_output_channel(
    bela_knyst_backend_ptr: *mut BelaKnystStarted,
    channel_index: libc::size_t,
) -> *const libc::c_float {
    if !bela_knyst_backend_ptr.is_null() {
        let output_channel = (*bela_knyst_backend_ptr).get_output_channel(channel_index);
        output_channel.as_ptr()
    } else {
        std::ptr::null()
    }
}

/// Drops the BelaKnystBackend
/// # Safety
/// This will drop a BelaKnystBackend if the pointer is not null. Don't give it anything
/// other than a pointer to a BelaKnystBackend gotten from `bela_knyst_backend_create`.
#[no_mangle]
pub unsafe extern "C" fn bela_knyst_destroy(bela_knyst_ptr: *mut BelaKnystStarted) {
    if !bela_knyst_ptr.is_null() {
        drop(Box::from_raw(bela_knyst_ptr));
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}
