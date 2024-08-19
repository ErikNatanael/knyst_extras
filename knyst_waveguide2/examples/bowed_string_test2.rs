use color_eyre::Result;
use knyst::{
    audio_backend::JackBackend,
    controller::print_error_handler,
    gen::{filter::one_pole::one_pole_lpf, random::random_lin},
    handles::{graph_output, Handle},
    prelude::*,
    sphere::{KnystSphere, SphereSettings},
};
use knyst_waveguide2::{
    bowed_string::bowed_waveguide, double_buffer_waveguide::waveguide,
    parallel_bpf_waveguide::parallel_bpf_waveguide,
};
use knyst_waveguide2::{half_sine_wt, split_string::split_waveguide};
// use knyst_waveguide2::{waveguide, white_noise};
use rand::{seq::SliceRandom, thread_rng, Rng};
fn main() -> Result<()> {
    // let mut backend = CpalBackend::new(CpalBackendOptions::default())?;
    let mut backend = JackBackend::new("split_string_knyst")?;
    let _sphere = KnystSphere::start(
        &mut backend,
        SphereSettings {
            num_inputs: 0,
            num_outputs: 2,
            ..Default::default()
        },
        print_error_handler,
    );

    for channel in 0..2 {
        std::thread::spawn(move || {
            let mut rng = thread_rng();
            // let exciter = half_sine_impulse().freq(200.).amp(0.2);
            let exciter = half_sine_wt().freq(2000.).amp(0.4);
            let exciter_input = one_pole_lpf().sig(exciter).cutoff_freq(5600.);
            // let bpf_mix = (sine().freq(0.5)* 0.35 + 0.71).powf(2.0);
            // let exciter_input = one_pole_hpf()
            // .sig(one_pole_lpf().sig(white_noise() * 0.1).cutoff_freq(100.))
            // .cutoff_freq(40.);
            let mut harmonic = 2;
            let wg = bowed_waveguide()
                .freq(200.)
                .exciter(exciter_input)
                // .feedback(1.005)
                .feedback(random_lin().freq(0.5) * 0.05 + 0.99)
                .damping(1000.)
                .lf_damping(6.)
                .position(1.0 / harmonic as f32)
                .bow_force(random_lin().freq(0.5) * 0.8 + 0.1)
                .bow_velocity(random_lin().freq(0.5) * 0.8 + 0.1)
                .stiffness(0.0);
            let mut position = 0.4;
            let sig = wg * 0.25;

            let mut freqs = vec![50., 100., 200., 400., 100., 500., 450.];
            let mut octave = 1.0;
            graph_output(0, sig.repeat_outputs(1));
            let mut i = 0;
            let mut bar_counter = 0;
            let mut accessible_notes = 3;
            let mut beat_time = 200;
            let melody_notes = [400.0f32, 450., 500., 550., 600., 650., 700., 750., 800.];
            loop {
                if i > 2 {
                    if rng.gen_bool(0.4) {
                        freqs[i] = *melody_notes[..accessible_notes].choose(&mut rng).unwrap();
                    }
                }
                let freq = freqs[i % freqs.len()] as f32 * octave;
                if i % 8 == 0 {
                    exciter.freq(freq * 2.75);
                    exciter.restart_trig();
                }
                wg.freq(freq);
                wg.damping(freq * 16. + 2000.);
                // wg.damping(rng.gen_range(200f32..10000_f32))
                //     .position(position);
                // if i % 3 == 0 {
                //     octave = *[0.25, 0.5, 1.0, 2.0].choose(&mut rng).unwrap();
                // }

                i += 1;
                if i >= freqs.len() {
                    if rng.gen_bool(0.5) {
                        i = 0;
                    } else {
                        i = 3;
                    }
                    if rng.gen_bool(0.05) {
                        octave = [(1.0, 0.5), (2.0, 0.3), (4.0, 0.2)]
                            .choose_weighted(&mut rng, |t| t.1)
                            .unwrap()
                            .0;
                    }
                    bar_counter += 1;
                    if bar_counter % 16 == 0 {
                        accessible_notes = (accessible_notes + 1);
                        if accessible_notes > melody_notes.len() {
                            accessible_notes = 3;
                        }
                        harmonic += 1;
                        if harmonic >= 9 {
                            harmonic = 3;
                        }
                        wg.position(1.0 / harmonic as f32);
                    }
                    if bar_counter % 64 == 0 {
                        beat_time /= 2;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(beat_time));
            }
        });
        // std::thread::sleep(std::time::Duration::from_millis(200 * 32));
    }

    // let mut rng = thread_rng();
    // // let exciter = half_sine_impulse().freq(200.).amp(0.2);
    // let exciter = half_sine_wt().freq(120.).amp(0.4);
    // let exciter_input = one_pole_lpf().sig(exciter).cutoff_freq(5600.);
    // // let bpf_mix = (sine().freq(0.5)* 0.35 + 0.71).powf(2.0);
    // // let exciter_input = one_pole_hpf()
    // // .sig(one_pole_lpf().sig(white_noise() * 0.1).cutoff_freq(100.))
    // // .cutoff_freq(40.);
    // let mut harmonic = 2;
    // let wg = split_waveguide()
    //     .freq(50.)
    //     .exciter(exciter_input)
    //     // .feedback(1.005)
    //     .feedback(1.007)
    //     .damping(1000.)
    //     .lf_damping(6.)
    //     .position(1.0 / harmonic as f32)
    //     .stop_amount(0.0)
    //     .stiffness(0.01);
    // let mut position = 0.4;
    // let sig = wg * 0.25;

    // graph_output(0, sig.repeat_outputs(1));
    // loop {
    //     exciter.amp(0.5);
    //     exciter.restart_trig();
    //     std::thread::sleep(std::time::Duration::from_millis(200));
    //     exciter.amp(0.2);
    //     exciter.restart_trig();
    //     std::thread::sleep(std::time::Duration::from_millis(400));
    // }
    loop {}

    Ok(())
}

// fn sine() -> NodeHandle<WavetableOscillatorOwnedHandle> {
//     wavetable_oscillator_owned(Wavetable::sine())
// }
fn sine() -> Handle<OscillatorHandle> {
    oscillator(WavetableId::cos())
}
