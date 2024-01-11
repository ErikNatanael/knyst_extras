use color_eyre::Result;
use knyst::audio_backend::JackBackend;
use knyst::controller::print_error_handler;
use knyst::envelope::Envelope;
use knyst::handles::{handle, Handle};
use knyst::prelude::*;
use knyst_visualiser::parameter::parameter;
use knyst_visualiser::probe;
use rand::Rng;
fn main() -> Result<()> {
    // Init Knyst
    // let mut backend = CpalBackend::new(CpalBackendOptions::default())?;
    let mut backend = JackBackend::new("Knyst<3JACK")?;
    let _sphere = KnystSphere::start(
        &mut backend,
        SphereSettings {
            num_inputs: 0,
            num_outputs: 2,
            ..Default::default()
        },
        print_error_handler,
    );

    std::thread::spawn(|| {
        let root_freq = parameter("root freq", 400.);
        for &freq in [1.0, 1.5, 5. / 4., 9. / 8.].iter().cycle() {
            let mut rng = rand::thread_rng();
            // for _ in 0..10 {
            let g = upload_graph(knyst_commands().default_graph_settings(), || {
                let freq = (sine().freq(
                    sine()
                        .freq(
                            sine()
                                .freq(0.01)
                                .range(0.02, rng.gen_range(0.05..0.3 as Sample)),
                        )
                        .range(0.0, 400.),
                ) * 100.0)
                    + root_freq;
                // let freq = sine().freq(0.5).range(200.0, 200.0 * 9.0 / 8.0);
                let node0 = sine();
                node0.freq(freq);
                let modulator = sine();
                modulator.freq(sine().freq(0.09) * -5.0 + 6.0);
                probe().input(modulator);
                graph_output(0, (node0 * modulator * 0.025).repeat_outputs(1));
            });
            graph_output(0, g);
            // }
            // new graph
            knyst_commands()
                .init_local_graph(knyst_commands().default_graph_settings().num_inputs(1));
            let root = graph_input(0, 1);
            let sig = sine().freq(freq as f32 * root).out("sig") * 0.25;
            let env = Envelope {
                points: vec![(1.0, 0.005), (0.0, 0.5)],
                stop_action: StopAction::FreeGraph,
                ..Default::default()
            };
            let sig = sig * handle(env.to_gen());
            // let sig = sig * handle(env.to_gen());

            graph_output(0, sig.repeat_outputs(1));
            // push graph to sphere
            let graph = knyst_commands().upload_local_graph().unwrap();
            graph.set(0, root_freq);
            // let sig = graph + static_sample_delay(48 * 500).input(graph);

            graph_output(0, graph.repeat_outputs(1));
            std::thread::sleep(std::time::Duration::from_millis(2500));
            g.free();
        }
    });

    // Init visualiser
    knyst_visualiser::init_knyst_visualiser();

    Ok(())
}
fn sine() -> Handle<OscillatorHandle> {
    oscillator(WavetableId::cos())
}
