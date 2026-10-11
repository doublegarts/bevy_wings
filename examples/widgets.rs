//! Interactive controls, text input, a progress indicator and snackbars.
mod support;
use bevy::prelude::*;
use dga_bevy_wings_framework::prelude::*;

fn main() {
    support::run("Wings: widgets", configure);
}
fn configure(app: &mut App) {
    app.add_systems(Startup, setup);
}
fn setup(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        @WingsScaffold
        Children [
            @WingsScrollColumn { @padding: 24.0, @gap: 16.0 }
            Children [
                @WingsText { @text: "Widget playground", @role: TypeRole::HeadlineMedium }
                --
                @WingsCheckbox { @label: "Checkbox" }
                --
                @WingsSwitch { @label: "Switch" }
                --
                @WingsRadioGroup { @gap: 8.0 }
                Children [
                    @WingsRadioButton { @label: "Option A", @checked: true }
                    --
                    @WingsRadioButton { @label: "Option B" }
                ]
                --
                @WingsSlider { @value: 0.5 }
                on(|change: On<ValueChange<f32>>| println!("Slider: {:.2}", change.value))
                --
                @WingsTextField { @label: "Type here", @value: "Hello" }
                on(|change: On<WingsTextChanged>| println!("Text: {}", change.value))
                --
                @WingsLinearProgress { @progress: 0.65 }
                --
                @WingsButton { @text: "Show snackbar" }
                on(|_: On<Activate>, mut messages: MessageWriter<WingsShowSnackbar>| {
                    messages.write(WingsShowSnackbar::new("Button activated").with_action("OK"));
                })
            ]
        ]
    });
}

#[test]
fn snackbar_message_spawns_feedback() {
    let mut app = support::app("widgets", true);
    configure(&mut app);
    app.update();
    app.world_mut()
        .write_message(WingsShowSnackbar::new("Example notification"));
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == "Example notification")
    );
}
