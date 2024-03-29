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
    bowed_string::bowed_waveguide, half_sine_wt, split_string::split_waveguide,
};
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

    for i in 0..1 {
        std::thread::spawn(move || {
            let mut rng = thread_rng();
            let mut freq_value = 100. * 2i32.pow(i) as f32;
            let freq_value = 100.;
            let freq = bus(1).set(0, freq_value);
            // let freq = (sine().freq(0.05) * 0.5 + 0.5).powf(2.0) * 1000. + 50.;
            let damping_range = freq * 10.;
            // let exciter = half_sine_impulse().freq(200.).amp(0.2);
            let exciter = half_sine_wt().freq(freq * 1.5).amp(0.4);
            let exciter_input = one_pole_lpf().sig(exciter).cutoff_freq(5600.);
            // let bpf_mix = (sine().freq(0.5)* 0.35 + 0.71).powf(2.0);
            // let exciter_input = one_pole_hpf()
            // .sig(one_pole_lpf().sig(white_noise() * 0.1).cutoff_freq(100.))
            // .cutoff_freq(40.);
            let mut harmonic = 2;
            let wg = bowed_waveguide()
                .freq(freq)
                .exciter(exciter_input)
                // .feedback(1.005)
                .feedback(0.95)
                // .damping(sine().freq(0.25) * damping_range + damping_range + freq)
                // .damping(sine().freq(0.1).range(freq * 2., 20000.))
                // .damping(freq * 9. * (1.0 + i as f32))
                .damping(freq * 20.)
                .lf_damping(6.)
                .position(0.1371)
                .bow_force(sine().freq(0.3).range(0.0, 1.0))
                // .bow_force(0.01)
                // .bow_velocity(sine().freq(2.7).range(0.0, 1.0))
                .bow_velocity(-0.25)
                // .bow_velocity(sine().freq(freq).range(0.0, 1.0))
                .stiffness(0.00);
            let mut position = 0.4;
            let sig = wg * 0.25;

            graph_output(0, sig.channels(2));
            exciter.restart_trig();
            // let sine = sine().freq(200.) * 0.1;
            // graph_output(1, sine);
            std::thread::sleep(std::time::Duration::from_millis(100 * i as u64));
            loop {
                // exciter.restart_trig();
                std::thread::sleep(std::time::Duration::from_millis(400));
                // freq_value *= 2.0;
                // if freq_value > 5000. {
                //     freq_value = 200. * i as f32;
                // }
                // freq.set(0, freq_value);
            }
        });
    }

    loop {
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    Ok(())
}

// fn sine() -> NodeHandle<WavetableOscillatorOwnedHandle> {
//     wavetable_oscillator_owned(Wavetable::sine())
// }
fn sine() -> Handle<OscillatorHandle> {
    oscillator(WavetableId::cos())
}
