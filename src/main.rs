use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy::render::camera::camera_system;
use bevy::sprite::{MaterialMesh2dBundle, Mesh2dHandle};
use bevy::window::{EnabledButtons, ExitCondition, PresentMode, WindowResolution};
use bevy_inspector_egui::bevy_egui::EguiContexts;
use bevy_inspector_egui::egui;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::prelude::*;
use bevy_screen_diagnostics::{ScreenDiagnosticsPlugin, ScreenFrameDiagnosticsPlugin};
use leafwing_input_manager::plugin::InputManagerPlugin;
use leafwing_input_manager::prelude::*;

const WINDOW_WIDTH: f32 = 1024.0;
const WINDOW_HEIGHT: f32 = 768.0;

const HALF_HEIGHT: f32 = WINDOW_HEIGHT / 2.0;
const HALF_WIDTH: f32 = WINDOW_WIDTH / 2.0;

const PLAYER_SPEED: f32 = 600.0;
const PLAYER_POSITION: Vec3 = Vec3::new(0.0, -HALF_HEIGHT * 0.86, 0.0);
const PLAYER_RADIUS: f32 = 25.0;

fn main() {
    App::new()
        // plugins
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::Immediate,
                resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
                resizable: false,
                enabled_buttons: EnabledButtons {
                    maximize: false,
                    ..default()
                },
                name: Some("BevyApp".to_string()),
                title: "LD55 - Summoning".to_string(),
                ..default()
            }),
            exit_condition: ExitCondition::OnPrimaryClosed,
            ..default()
        }))
        .add_plugins(
            WorldInspectorPlugin::default().run_if(input_toggle_active(false, KeyCode::F1)),
        )
        .add_plugins(ScreenDiagnosticsPlugin::default())
        .add_plugins(ScreenFrameDiagnosticsPlugin)
        .add_plugins(InputManagerPlugin::<Action>::default())
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(50.0))
        .add_plugins(RapierDebugRenderPlugin::default())
        // systems
        .add_systems(Startup, setup)
        .add_systems(Update, handle_actions)
        // .add_systems(Update, handle_collisions)
        // .add_systems(Update, dev_tools_system)
        // resources
        // start
        .run();
}

#[derive(Component)]
struct Camera;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Pet;

#[derive(Component)]
struct Enemy;

#[derive(Component)]
struct Health {
    current: u32,
    max: u32,
}

#[derive(Component)]
struct Xp(u32);

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
enum Action {
    // movement
    Up,
    Down,
    Left,
    Right,
    // abilities
    Fire,
}

impl Action {
    fn default_input_map() -> InputMap<Self> {
        let mut input_map = InputMap::default();

        input_map.insert_one_to_many(Self::Up, [KeyCode::ArrowUp, KeyCode::KeyW]);
        input_map.insert(Self::Up, GamepadButtonType::DPadUp);

        input_map.insert_one_to_many(Self::Down, [KeyCode::ArrowDown, KeyCode::KeyS]);
        input_map.insert(Self::Down, GamepadButtonType::DPadDown);

        input_map.insert_one_to_many(Self::Left, [KeyCode::ArrowLeft, KeyCode::KeyA]);
        input_map.insert(Self::Left, GamepadButtonType::DPadLeft);

        input_map.insert_one_to_many(Self::Right, [KeyCode::ArrowRight, KeyCode::KeyD]);
        input_map.insert(Self::Right, GamepadButtonType::DPadRight);

        input_map.insert(Self::Fire, KeyCode::Space);
        input_map.insert(Self::Fire, GamepadButtonType::South);

        input_map
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // configure and spawn the camera
    let mut camera = Camera2dBundle {
       // transform: Transform::from_xyz(100.0, 200.0, 0.0),
        ..default()
    };
    camera.projection.scale = 2.0;
    // camera.transform.rotate_z(0f32.to_radians());
    commands.spawn((camera, Camera));

    // create the top
    commands
        .spawn(Collider::cuboid(HALF_WIDTH, 1.0))
        .insert(Name::new("Wall_Top"))
        .insert(TransformBundle::from(Transform::from_xyz(
            0.0,
            HALF_HEIGHT,
            0.0,
        )));

    // create the left wall
    commands
        .spawn(Collider::cuboid(1.0, HALF_HEIGHT))
        .insert(Name::new("Wall_Left"))
        .insert(TransformBundle::from(Transform::from_xyz(
            -HALF_WIDTH,
            0.0,
            0.0,
        )));

    // create the right wall
    commands
        .spawn(Collider::cuboid(1.0, HALF_HEIGHT))
        .insert(Name::new("Wall_Right"))
        .insert(TransformBundle::from(Transform::from_xyz(
            HALF_WIDTH, 0.0, 0.0,
        )));

    // create the bottom
    commands
        .spawn(Collider::cuboid(HALF_WIDTH, 1.0))
        .insert(Name::new("Wall_Bottom"))
        .insert(TransformBundle::from(Transform::from_xyz(
            0.0,
            -HALF_HEIGHT,
            0.0,
        )));

    let player_shape = Mesh2dHandle(meshes.add(Circle { radius:PLAYER_RADIUS }));
    let player_material = materials.add(Color::rgb(0.0, 255.0, 0.0));

    // configure and spawn the player
    commands
        .spawn(RigidBody::KinematicPositionBased)
        .insert(Name::new("Player"))
        .insert(KinematicCharacterController {
            ..default()
        })
        .insert(Collider::ball(PLAYER_RADIUS))
        .insert(GravityScale(0.0))
        .insert(Dominance::group(5))
        .insert(TransformBundle::from(Transform {
            translation: PLAYER_POSITION,
            ..default()
        }))
        .insert(MaterialMesh2dBundle {
            mesh: player_shape,
            material: player_material,
            ..default()
        })
        .insert(InputManagerBundle::with_map(Action::default_input_map()))
        .insert(Health {
            current: 10,
            max: 10,
        })
        .insert(Xp(0))
        .insert(Player);
}

fn handle_actions(
    time: Res<Time>,
    action_query: Query<&ActionState<Action>, With<Player>>,
    mut controllers: Query<&mut KinematicCharacterController>,
) {
    for action_state in action_query.iter() {
        let mut new_x = 0.0;
        let mut new_y = 0.0;

        let speed = PLAYER_SPEED * time.delta_seconds();

        if action_state.pressed(&Action::Up) {
            new_y = speed;
        }

        if action_state.pressed(&Action::Down) {
            new_y = -speed;
        }

        if action_state.pressed(&Action::Left) {
            new_x = -speed;
        }

        if action_state.pressed(&Action::Right) {
            new_x = speed;
        }

        for mut controller in controllers.iter_mut() {
            controller.translation = Some(Vec2::new(new_x, new_y));
        }
    }
}
