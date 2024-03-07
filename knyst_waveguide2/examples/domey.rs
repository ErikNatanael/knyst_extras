use std::time::Duration;

use anyhow::Result;
use knyst::{
    audio_backend::JackBackend,
    controller::print_error_handler,
    envelope::Envelope,
    gen::{delay::allpass_feedback_delay, filter::one_pole::one_pole_lpf, random::random_lin},
    handles::{graph_output, handle, Handle},
    modal_interface::knyst_commands,
    prelude::*,
    sphere::{KnystSphere, SphereSettings},
    trig::interval_trig,
};
use knyst_airwindows::galactic;
use knyst_waveguide2::{half_sine_wt, waveguide};
use rand::seq::SliceRandom;
use rand::{random, thread_rng, Rng}; // 0.7.2
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

    for channel in 0..2 {
        std::thread::spawn(move || {
            let mut rng = thread_rng();
            knyst_commands().init_local_graph(knyst_commands().default_graph_settings());
            for freq in [50, 100, 200, 300, 400, 500, 600, 700, 800].iter() {
                let exciter = half_sine_wt()
                    .freq(2000.)
                    .amp(random_lin().freq(0.5) * 0.2 + 0.05)
                    .restart(interval_trig().interval(random_lin().freq(0.3) * 10.0 + 0.3));
                let exciter_input = one_pole_lpf().sig(exciter).cutoff_freq(2600.);
                // let exciter_input = one_pole_hpf()
                // .sig(one_pole_lpf().sig(white_noise() * 0.1).cutoff_freq(100.))
                // .cutoff_freq(40.);
                let wg = waveguide()
                    .freq(*freq as f32)
                    .exciter(exciter_input)
                    .feedback((1.03 - random_lin().freq(1.5) * 0.1).powf(2.0))
                    .damping(random_lin().freq(0.1) * 10000. + 3000.)
                    .lf_damping(6.)
                    .position(0.25 + random_lin().freq(0.05) * 0.25)
                    .stiffness(0.0);
                let sig = wg * 0.25;
                // let sig = sine().freq(*freq as Sample).out("sig") * 0.25;
                // let env = Envelope {
                //     points: vec![(1.0, 0.001), (0.0, 0.2)],
                //     ..Default::default()
                // };
                // let sig = sig
                //     * handle(env.to_gen()).set(
                //         "restart",
                //         interval_trig().interval(rng.gen_range(0.5..10.5)),
                //     );
                graph_output(0, sig);
            }
            let sig = knyst_commands().upload_local_graph().unwrap();
            let verb = galactic()
                .size(1.0)
                .brightness(0.9)
                .detune(0.2)
                .mix(random_lin().freq(0.2))
                .replace(0.1);
            // .input(sig * 0.125);
            // .input(sig * 0.125 + graph_input(0, 1));
            verb.left(sig + graph_input(0, 1));
            verb.right(sig + graph_input(0, 1));
            // verb.input(sig * 0.125);
            let sig = verb;
            // let sig = verb * 0.25 + (sig * 0.25);
            graph_output(channel, sig.out(0));
        });
    }

    let exciter = half_sine_wt()
        .freq(2000.)
        .amp(random_lin().freq(0.5) * 0.2 + 0.05)
        .restart(interval_trig().interval(random_lin().freq(0.3) * 10.0 + 2.3));
    let exciter_input = one_pole_lpf().sig(exciter).cutoff_freq(2600.);
    // let exciter_input = one_pole_hpf()
    // .sig(one_pole_lpf().sig(white_noise() * 0.1).cutoff_freq(100.))
    // .cutoff_freq(40.);
    let wg = waveguide()
        .freq(200. as f32)
        .exciter(exciter_input)
        .feedback((1.03 - random_lin().freq(1.5) * 0.1).powf(2.0))
        .damping(random_lin().freq(0.1) * 10000. + 3000.)
        .lf_damping(6.)
        .position(0.25 + random_lin().freq(0.05) * 0.25)
        .stiffness(0.0);
    let delay_input_sig = wg * 0.25;
    let mut delays = vec![];
    for channel in 0..2 {
        if channel == 0 {
            let delay = allpass_feedback_delay(48000)
                .delay_time(0.2)
                .feedback(0.2)
                .input(delay_input_sig);
            graph_output(channel, delay);
            delays.push(delay);
        } else {
            let delay = allpass_feedback_delay(48000)
                .delay_time(0.2)
                .feedback(0.5)
                .input(*delays.last().unwrap());
            graph_output(channel, delay);
        }
    }
    std::thread::spawn(move || {
        let mut rng = thread_rng();
        let freqs = vec![50, 100, 150, 200, 250, 300, 350, 300];
        loop {
            let new_freq = freqs.choose(&mut rng).unwrap();
            wg.freq(*new_freq as f32);
            std::thread::sleep(Duration::from_secs_f32(rng.gen_range(2.5..10.0)));
        }
    });

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
        print!(": ");
        match std::io::stdin().read_line(&mut input) {
            Ok(_) => {
                let trimmed_input = input.trim();
                if let Ok(param) = trimmed_input.parse::<usize>() {
                    input.clear();
                    if let Ok(_) = std::io::stdin().read_line(&mut input) {}
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
