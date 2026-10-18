//! Stress benchmarks: very large widget counts in a headless app.
//!
//! ```text
//! cargo bench -p bevy_wings_ui_widgets --bench stress
//! ```
//!
//! Every case reports the median of several runs in milliseconds. `cargo test` runs the same
//! code once with small counts, so the benchmark keeps compiling and working.

use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use bevy_wings_ui_theme::prelude::*;
use bevy_wings_ui_widgets::prelude::*;

fn app(design: WingsDesign) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        bevy::input::InputPlugin,
    ));
    app.init_asset::<Image>();
    app.add_plugins((
        bevy::ui_widgets::ButtonPlugin,
        bevy::ui_widgets::CheckboxPlugin,
        bevy::ui_widgets::RadioGroupPlugin,
        bevy::ui_widgets::SliderPlugin,
    ));
    let mut theme = WingsTheme::for_design(design);
    theme.motion.theme_transition = None;
    app.add_plugins((
        WingsThemePlugin::with_theme(theme),
        WingsWidgetsPlugin::headless(),
    ));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        16,
    )));
    app.update();
    app
}

/// Spawns `count` mixed widgets: buttons, switches, checkboxes and text.
fn spawn_widgets(app: &mut App, count: usize) {
    for i in 0..count {
        let text = format!("Item {i}");
        match i % 4 {
            0 => {
                app.world_mut()
                    .spawn_scene(bsn! { @WingsButton { @text: text } })
                    .unwrap();
            }
            1 => {
                app.world_mut().spawn_scene(bsn! { @WingsSwitch }).unwrap();
            }
            2 => {
                app.world_mut()
                    .spawn_scene(bsn! { @WingsCheckbox { @label: text } })
                    .unwrap();
            }
            _ => {
                app.world_mut()
                    .spawn_scene(bsn! { @WingsText { @text: text } })
                    .unwrap();
            }
        }
    }
}

fn time(f: impl FnOnce()) -> f64 {
    let start = Instant::now();
    f();
    start.elapsed().as_secs_f64() * 1000.0
}

fn median(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}

fn report(name: &str, samples: Vec<f64>) {
    println!("{name:<44} {:>10.2} ms", median(samples));
}

fn bench_widgets(count: usize, runs: usize) {
    let mut spawn = Vec::new();
    let mut idle = Vec::new();
    let mut theme = Vec::new();
    let mut design = Vec::new();
    for _ in 0..runs {
        let mut app = app(WingsDesign::Material);
        spawn.push(time(|| {
            spawn_widgets(&mut app, count);
            app.update();
            app.update();
        }));
        app.update();
        idle.push(time(|| app.update()));
        theme.push(time(|| {
            app.world_mut().resource_mut::<WingsTheme>().toggle_dark();
            app.update();
        }));
        design.push(time(|| {
            app.world_mut()
                .resource_mut::<WingsTheme>()
                .set_design(WingsDesign::Ios);
            app.update();
            app.update();
        }));
    }
    report(&format!("spawn {count} widgets (+2 frames)"), spawn);
    report(&format!("idle frame, {count} widgets"), idle);
    report(&format!("light/dark switch, {count} widgets"), theme);
    report(&format!("Material to iOS re-skin, {count} widgets"), design);
}

fn bench_virtual_list(count: usize, frames: usize) {
    let mut app = app(WingsDesign::Material);
    let list = app
        .world_mut()
        .spawn(WingsVirtualList::new(count, 52.0, |i| {
            let text = format!("Row {i}");
            Box::new(bsn! { @WingsListItem { @headline: text } })
        }))
        .id();
    app.update();
    app.update();
    let mut samples = Vec::new();
    for frame in 0..frames {
        app.world_mut().get_mut::<ScrollPosition>(list).unwrap().0.y = frame as f32 * 40.0;
        samples.push(time(|| app.update()));
    }
    let rows = app.world().get::<WingsVirtualRows>(list).unwrap().len();
    assert!(rows < 64, "a virtual list keeps only visible rows ({rows})");
    report(
        &format!("virtual list of {count}, scrolling frame"),
        samples,
    );
}

fn main() {
    let full = std::env::args().any(|a| a == "--bench");
    if full {
        println!("Wings stress benchmarks (headless, median of runs)\n");
        for count in [1_000, 10_000] {
            bench_widgets(count, 5);
        }
        bench_virtual_list(100_000, 120);
    } else {
        // `cargo test`: a quick smoke run.
        bench_widgets(40, 1);
        bench_virtual_list(1_000, 5);
    }
}
