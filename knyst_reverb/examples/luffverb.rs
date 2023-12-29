use anyhow::Result;
use knyst::{
    audio_backend::JackBackend,
    controller::print_error_handler,
    envelope::Envelope,
    handles::{graph_output, handle, Handle},
    modal_interface::knyst,
    prelude::*,
    sphere::{KnystSphere, SphereSettings},
    trig::interval_trig,
};
use knyst_reverb::luff_verb;
use rand::{thread_rng, Rng};
fn main() -> Result<()> {
    // let mut backend = CpalBackend::new(CpalBackendOptions::default())?;
    let mut backend = JackBackend::new("Knyst<3JACK")?;
    let _sphere = KnystSphere::start(
        &mut backend,
        SphereSettings {
            num_inputs: 1,
            num_outputs: 2,
            ..Default::default()
        },
        print_error_handler,
    );

    let mut rng = thread_rng();
    // Put all the different sine oscillators inside a graph
    let sine_graph = upload_graph(knyst().default_graph_settings(), || {
        for freq_mul in [3, 4, 5, 6, 7, 8].iter() {
            // Get the root frequency from the graph input
            let root_freq = graph_input(0, 1);
            let sig = sine().freq(*freq_mul as f32 * root_freq).out("sig") * 0.25;
            let env = Envelope {
                points: vec![(1.0, 0.001), (0.0, 0.2)],
                ..Default::default()
            };
            // Restart the envelope at a different random interval for each overtone
            let sig = sig
                * handle(env.to_gen()).set(
                    "restart",
                    interval_trig().interval(rng.gen_range(1.5f32..3.5)),
                );
            graph_output(0, sig);
        }
    });
    // Set the root freq to an initial value of 200. Hz
    sine_graph.set(0, 200.);
    let verb = luff_verb(2350 * 48, 0.65, 0.3)
        .lowpass(7000.)
        .damping(4000.);
    // Connect the sine wave graph output and the first top level graph input to the reverb input
    verb.input(sine_graph * 0.125 + graph_input(0, 1));
    let sig = verb * 0.5;
    // Send the reverb output to both left and right channels.
    graph_output(0, sig.repeat_outputs(1));

    let mut input = String::new();
    loop {
        match std::io::stdin().read_line(&mut input) {
            Ok(n) => {
                println!("{} bytes read", n);
                println!("{}", input.trim());
                let input = input.trim();
                if let Ok(freq) = input.parse::<usize>() {
                    sine_graph.set(0, freq as f32);
                } else if input == "q" {
                    break;
                }
            }
            Err(error) => println!("error: {}", error),
        }
        input.clear();
    }
    Ok(())
}

// fn sine() -> NodeHandle<WavetableOscillatorOwnedHandle> {
//     wavetable_oscillator_owned(Wavetable::sine())
// }
fn sine() -> Handle<OscillatorHandle> {
    oscillator(WavetableId::cos())
}
