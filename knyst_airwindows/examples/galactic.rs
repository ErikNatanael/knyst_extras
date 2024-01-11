use anyhow::Result;
use knyst::{
    audio_backend::JackBackend,
    controller::print_error_handler,
    envelope::Envelope,
    handles::{graph_output, handle, Handle},
    modal_interface::knyst_commands,
    prelude::*,
    sphere::{KnystSphere, SphereSettings},
    trig::interval_trig,
};
use knyst_airwindows::galactic;
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
    knyst_commands().init_local_graph(knyst_commands().default_graph_settings());
    for freq in [300, 400, 500, 600, 700, 800].iter() {
        let sig = sine().freq(*freq as Sample).out("sig") * 0.25;
        let env = Envelope {
            points: vec![(1.0, 0.001), (0.0, 0.2)],
            ..Default::default()
        };
        let sig = sig
            * handle(env.to_gen()).set(
                "restart",
                interval_trig().interval(rng.gen_range(0.5..10.5)),
            );
        graph_output(0, sig);
    }
    let sig = knyst_commands().upload_local_graph().unwrap();
    let verb = galactic()
        .size(1.0)
        .brightness(0.9)
        .detune(0.2)
        .mix(0.5)
        .replace(0.1);
    // .input(sig * 0.125);
    // .input(sig * 0.125 + graph_input(0, 1));
    verb.left(sig + graph_input(0, 1));
    verb.right(sig + graph_input(0, 1));
    // verb.input(sig * 0.125);
    let sig = verb;
    // let sig = verb * 0.25 + (sig * 0.25);
    graph_output(0, sig);

    // std::thread::sleep(std::time::Duration::from_millis(150));
    // for &freq in [400, 600, 500].iter().cycle() {
    //     // new graph
    //     commands().init_local_graph(commands().default_graph_settings());
    //     let sig = sine().freq(freq as f32).out("sig") * 0.25;
    //     let env = Envelope {
    //         points: vec![(1.0, 0.005), (0.0, 0.5)],
    //         stop_action: StopAction::FreeGraph,
    //         ..Default::default()
    //     };
    //     let sig = sig * handle(env.to_gen());

    //     graph_output(0, sig.repeat_outputs(1));
    //     // push graph to sphere
    // let graph = commands().upload_local_graph();

    //     // graph_output(0, graph);
    //     verb.input(graph.out(0) * 1.1);
    //     std::thread::sleep(std::time::Duration::from_millis(2500));
    // }

    // graph_output(0, (sine(wt).freq(200.)).repeat_outputs(1));

    // let inspection = commands().request_inspection();
    // let inspection = inspection.recv().unwrap();
    // dbg!(inspection);

    let mut input = String::new();
    loop {
        println!("1. Replace\n2.Brightness\n3.Detune\n4.Bigness\n5.Dry/wet\n");
        print!(": ");
        match std::io::stdin().read_line(&mut input) {
            Ok(_) => {
                let trimmed_input = input.trim();
                if let Ok(param) = trimmed_input.parse::<usize>() {
                    println!("param: {param}");
                    input.clear();
                    if let Ok(_) = std::io::stdin().read_line(&mut input) {
                        let trimmed_input = input.trim();
                        dbg!(trimmed_input);
                        if let Ok(value) = trimmed_input.parse::<f32>() {
                            println!("Setting {param} to {value}");
                            match param {
                                1 => {
                                    verb.replace(value);
                                }
                                2 => {
                                    verb.brightness(value);
                                }
                                3 => {
                                    verb.detune(value);
                                }
                                4 => {
                                    verb.size(value);
                                }
                                5 => {
                                    verb.mix(value);
                                }
                                _ => (),
                            };
                        }
                    }
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
