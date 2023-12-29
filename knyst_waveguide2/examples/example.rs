use color_eyre::Result;
use knyst::{
    audio_backend::JackBackend,
    controller::print_error_handler,
    gen::{filter::one_pole::one_pole_lpf, random::random_lin},
    handles::{graph_output, Handle},
    prelude::*,
    sphere::{KnystSphere, SphereSettings},
};
use knyst_waveguide2::{double_buffer_waveguide::waveguide, parallel_bpf_waveguide::parallel_bpf_waveguide};
use knyst_waveguide2::half_sine_wt;
// use knyst_waveguide2::{waveguide, white_noise};
use rand::{seq::SliceRandom, thread_rng, Rng};
fn main() -> Result<()> {
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

    let mut rng = thread_rng();
    // let exciter = half_sine_impulse().freq(200.).amp(0.2);
    let exciter = half_sine_wt().freq(2000.).amp(0.4);
    let exciter_input = one_pole_lpf().sig(exciter).cutoff_freq(2600.);
    let bpf_freq = sine().freq(0.1).range(100., 2000.);
    let bpf_mix = 0.15;
    // let bpf_mix = (sine().freq(0.5)* 0.35 + 0.71).powf(2.0);
    // let exciter_input = one_pole_hpf()
    // .sig(one_pole_lpf().sig(white_noise() * 0.1).cutoff_freq(100.))
    // .cutoff_freq(40.);
    let wg = parallel_bpf_waveguide()
        .freq(200.)
        .exciter(exciter_input)
        .feedback(1.05)
        .damping(6000.)
        .lf_damping(6.)
        .position(0.25)
        .bpf_freq(bpf_freq)
        .bpf_mix(bpf_mix)
        .stiffness(0.0);
    let mut position = 0.4;
    let sig = wg * 0.25;

    let mut freqs = [100, 200, 400, 100, 500, 450].iter().cycle();
    let mut octave = 1.0;
    graph_output(0, sig.repeat_outputs(1));
    let mut i = 0;
    loop {
        let freq = *freqs.next().unwrap() as f32 * octave;
        if i % 8 == 0 {
            exciter.freq(freq * 0.75);
            exciter.restart_trig();
        }
        wg.freq(100.);
        // wg.bpf_freq(freq);
        let bpf_mix = rng.gen_range(0.0..0.6);
        // dbg!(bpf_mix);
        // wg.bpf_mix(bpf_mix);
        // wg.damping(rng.gen_range(200f32..10000_f32))
        //     .position(position);
        position += rng.gen_range(-0.05f32..0.05);
        position = position.clamp(0.1, 0.5);
        if i % 3 == 0 {
            octave = *[0.25, 0.5, 1.0, 2.0].choose(&mut rng).unwrap();
        }

        i += 1;
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }

    Ok(())
}

// fn sine() -> NodeHandle<WavetableOscillatorOwnedHandle> {
//     wavetable_oscillator_owned(Wavetable::sine())
// }
fn sine() -> Handle<OscillatorHandle> {
    oscillator(WavetableId::cos())
}
