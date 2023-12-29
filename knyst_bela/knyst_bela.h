#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

struct BelaKnystBackend;

extern "C" {

/// Creates an opaque pointer to the BelaKnystBackend
///
/// # Safety
/// Make sure you destroy the SynthesisInterface with `bela_knyst_backend_destroy`
/// once you are done with it.
BelaKnystBackend *bela_knyst_backend_create(float sample_rate,
                                            size_t block_size,
                                            size_t num_inputs,
                                            size_t num_outputs);

/// Run updates (non realtime thread safe)
///
/// # Safety
/// Only call with a pointer received from `bela_knyst_backend_create`
void bela_knyst_backend_update(BelaKnystBackend *bela_knyst_backend_ptr);

/// Set an input channel. Run before process_block
///
/// # Safety
/// Only call with a pointer received from `bela_knyst_backend_create`
void bela_knyst_backend_set_input_channel(BelaKnystBackend *bela_knyst_backend_ptr,
                                          size_t channel_index,
                                          const float *input_channel_ptr,
                                          size_t block_size);

/// Process one block of audio
///
/// # Safety
/// Only call with a pointer received from `bela_knyst_backend_create`
void bela_knyst_backend_process_block(BelaKnystBackend *bela_knyst_backend_ptr);

/// Get an output channel. Run after process_block
///
/// # Safety
/// Only call with a pointer received from `bela_knyst_backend_create`. Use immediately after receiving it and then never use the pointer again.
const float *bela_knyst_backend_get_output_channel(BelaKnystBackend *bela_knyst_backend_ptr,
                                                   size_t channel_index);

/// Drops the BelaKnystBackend
/// # Safety
/// This will drop a BelaKnystBackend if the pointer is not null. Don't give it anything
/// other than a pointer to a BelaKnystBackend gotten from `bela_knyst_backend_create`.
void bela_knyst_backend_destroy(BelaKnystBackend *bela_knyst_backend_ptr);

} // extern "C"
