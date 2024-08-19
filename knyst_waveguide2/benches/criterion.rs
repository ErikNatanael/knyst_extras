use criterion::{black_box, criterion_group, criterion_main, Criterion};
use knyst::{BlockSize, SampleRate};
use knyst_waveguide2::{
    bowed_string::{BowedWaveguide, BowedWaveguideOversampled},
    bowed_string_simplified::BowedWaveguideSimplified,
};

pub fn bowed_vs_simplified(c: &mut Criterion) {
    const BLOCK: usize = 32;
    let mut frequency = 440.0;
    let sample_rate = 48000.0;
    /*
    let mut bwg = BowedWaveguide::new();
    bwg.init(SampleRate(sample_rate));
    let mut output = [0.0; BLOCK];
    let mut reset_trig = [0.0; BLOCK];
    reset_trig[0] = 1.0;
    bwg.process(
        &[0.; BLOCK],
        &[frequency; BLOCK],
        &[0.25; BLOCK],
        &[0.99; BLOCK],
        &[0.; BLOCK],
        &[7000.; BLOCK],
        &[5.; BLOCK],
        &[0.; BLOCK],
        &[0.5; BLOCK],
        &[0.65; BLOCK],
        &reset_trig,
        &mut output,
        SampleRate(sample_rate),
    );
    reset_trig[0] = 0.;
    c.bench_function("bowed", |b| {
        b.iter(|| {
            bwg.process(
                &[0.; BLOCK],
                &[frequency; BLOCK],
                &[0.25; BLOCK],
                &[0.99; BLOCK],
                &[0.; BLOCK],
                &[7000.; BLOCK],
                &[5.; BLOCK],
                &[0.; BLOCK],
                &[0.5; BLOCK],
                &[0.65; BLOCK],
                &reset_trig,
                &mut output,
                SampleRate(sample_rate),
            );

            black_box(output);
        });
    });
    reset_trig[0] = 1.;
    c.bench_function("bowed and reset", |b| {
        b.iter(|| {
            bwg.process(
                &[0.; BLOCK],
                &[frequency; BLOCK],
                &[0.25; BLOCK],
                &[0.99; BLOCK],
                &[0.; BLOCK],
                &[7000.; BLOCK],
                &[5.; BLOCK],
                &[0.; BLOCK],
                &[0.5; BLOCK],
                &[0.65; BLOCK],
                &reset_trig,
                &mut output,
                SampleRate(sample_rate),
            );

            black_box(output);
        });
    });
    */

    let mut bwg = BowedWaveguideOversampled::new();
    bwg.init(SampleRate(sample_rate), BlockSize(BLOCK));
    let mut output = [0.0; BLOCK];
    let mut reset_trig = [0.0; BLOCK];
    reset_trig[0] = 1.0;
    bwg.process(
        &[0.; BLOCK],
        &[frequency; BLOCK],
        &[0.25; BLOCK],
        &[0.99; BLOCK],
        &[0.; BLOCK],
        &[7000.; BLOCK],
        &[5.; BLOCK],
        &[0.; BLOCK],
        &[0.5; BLOCK],
        &[0.65; BLOCK],
        &reset_trig,
        &mut output,
        SampleRate(sample_rate),
    );
    reset_trig[0] = 0.;
    c.bench_function("bowed oversampled", |b| {
        b.iter(|| {
            bwg.process(
                &[0.; BLOCK],
                &[frequency; BLOCK],
                &[0.25; BLOCK],
                &[0.99; BLOCK],
                &[0.; BLOCK],
                &[7000.; BLOCK],
                &[5.; BLOCK],
                &[0.; BLOCK],
                &[0.5; BLOCK],
                &[0.65; BLOCK],
                &reset_trig,
                &mut output,
                SampleRate(sample_rate),
            );

            black_box(output);
        });
    });
    reset_trig[0] = 1.;
    frequency = 440.;
    c.bench_function("bowed and reset oversampled", |b| {
        b.iter(|| {
            bwg.process(
                &[0.; BLOCK],
                &[frequency; BLOCK],
                &[0.25; BLOCK],
                &[0.99; BLOCK],
                &[0.; BLOCK],
                &[7000.; BLOCK],
                &[5.; BLOCK],
                &[0.; BLOCK],
                &[0.5; BLOCK],
                &[0.65; BLOCK],
                &reset_trig,
                &mut output,
                SampleRate(sample_rate),
            );
            frequency += 1.;
            if frequency > 2000. {
                frequency = 440.;
            }

            black_box(output);
        });
    });

    /*
    let mut bwg = BowedWaveguideSimplified::new();
    bwg.init(SampleRate(sample_rate));
    let mut output = [0.0; BLOCK];
    let mut reset_trig = [0.0; BLOCK];
    reset_trig[0] = 1.0;
                frequency = 440.;
    bwg.process(
        &[0.; BLOCK],
        &[frequency; BLOCK],
        &[0.25; BLOCK],
        &[0.99; BLOCK],
        &[0.; BLOCK],
        &[7000.; BLOCK],
        &[5.; BLOCK],
        &[0.; BLOCK],
        &[0.5; BLOCK],
        &[0.65; BLOCK],
        &reset_trig,
        &mut output,
        SampleRate(sample_rate),
    );
    reset_trig[0] = 0.;
    c.bench_function("bowed simplified", |b| {
        b.iter(|| {
            bwg.process(
                &[0.; BLOCK],
                &[frequency; BLOCK],
                &[0.25; BLOCK],
                &[0.99; BLOCK],
                &[0.; BLOCK],
                &[7000.; BLOCK],
                &[5.; BLOCK],
                &[0.; BLOCK],
                &[0.5; BLOCK],
                &[0.65; BLOCK],
                &reset_trig,
                &mut output,
                SampleRate(sample_rate),
            );

            black_box(output);
        });
    });
    reset_trig[0] = 1.;
    c.bench_function("bowed simplified and reset", |b| {
        b.iter(|| {
            bwg.process(
                &[0.; BLOCK],
                &[frequency; BLOCK],
                &[0.25; BLOCK],
                &[0.99; BLOCK],
                &[0.; BLOCK],
                &[7000.; BLOCK],
                &[5.; BLOCK],
                &[0.; BLOCK],
                &[0.5; BLOCK],
                &[0.65; BLOCK],
                &reset_trig,
                &mut output,
                SampleRate(sample_rate),
            );

            black_box(output);
        });
    });
        */
}

// criterion_group!(benches, phase_float_or_uint);
// criterion_group!(benches, envelope_segments);
criterion_group!(benches, bowed_vs_simplified);

criterion_main!(benches);
