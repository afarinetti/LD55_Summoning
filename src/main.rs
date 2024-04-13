use std::ptr::addr_of_mut;
use bevy::input::common_conditions::input_toggle_active;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::sprite::{MaterialMesh2dBundle, Mesh2dHandle};
use bevy::utils::tracing::event;
use bevy::window::{EnabledButtons, ExitCondition, PresentMode, WindowResolution};
use bevy_inspector_egui::egui::debug_text::print;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_xpbd_2d::prelude::*;
use bevy_screen_diagnostics::{ScreenDiagnosticsPlugin, ScreenFrameDiagnosticsPlugin};
use leafwing_input_manager::plugin::InputManagerPlugin;
use leafwing_input_manager::prelude::*;
use rand::random;

const WINDOW_WIDTH: f32 = 1024.0;
const WINDOW_HEIGHT: f32 = 768.0;

const HALF_HEIGHT: f32 = WINDOW_HEIGHT / 2.0;
const HALF_WIDTH: f32 = WINDOW_WIDTH / 2.0;

const PLAYER_SPEED: f32 = 400.0;
const PLAYER_RADIUS: f32 = 25.0;
const PLAYER_POSITION: Vec3 = Vec3::new(0.0, 0.0, 0.0);

const ENEMY_SPEED: f32 = PLAYER_SPEED * 0.5;
const ENEMY_RADIUS: f32 = PLAYER_RADIUS * 1.5;
const ENEMY_POSITION: Vec3 = Vec3::new(WINDOW_WIDTH, WINDOW_HEIGHT, 0.0);

const MINION_SPEED: f32 = PLAYER_SPEED * 1.5;
const MINION_RADIUS: f32 = PLAYER_RADIUS / 2.0;


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
        }).set(LogPlugin {
            filter: "info,wgpu=error,ld55_summoning=debug".into(),
            level: bevy::log::Level::DEBUG,
            ..default()
        }),)
        .add_plugins(
            WorldInspectorPlugin::default().run_if(input_toggle_active(false, KeyCode::F1)),
        )
        .add_plugins(ScreenDiagnosticsPlugin::default())
        .add_plugins(ScreenFrameDiagnosticsPlugin)
        .add_plugins(InputManagerPlugin::<Action>::default())
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(PhysicsDebugPlugin::default())
        // events
        .add_event::<SpawnMinionEvent>()
        // systems
        .add_systems(Startup, setup)
        .add_systems(Startup, spawn_enemy.after(setup))
        .add_systems(Update, bevy::window::close_on_esc)
        .add_systems(Update, minion_spawner)
        .add_systems(Update, handle_actions)
        .add_systems(Update, enemy_movement)
        .add_systems(Update, minion_movement)
        // .add_systems(Update, handle_collisions)
        // .add_systems(Update, dev_tools_system)
        // resources
        // start
        .run();
}

#[derive(Component)]
struct CameraMarker;

#[derive(Resource)]
struct GameSettings {
    current_level: u32,
    max_time_seconds: u32,
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Minion;

#[derive(Component)]
struct Enemy;

#[derive(Component)]
struct Health {
    current: u32,
    max: u32,
}

#[derive(Component)]
struct Xp(u32);

#[derive(Event)]
struct SpawnMinionEvent(f32);

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
    // configure and spawn theS camera
    let mut camera = Camera2dBundle {
        // camera: Camera {
        //     viewport: Some(Viewport {
        //         physical_position: UVec2::new(0, 0),
        //         physical_size: UVec2::new(256, 256),
        //         ..default()
        //     }),
        //     ..default()
        // },
        ..default()
    };
    // camera.projection.scale = 2.0;
    // camera.transform.rotate_z(0f32.to_radians());
    commands.spawn((camera, CameraMarker));

    // create the top
    commands
        .spawn(RigidBody::Static)
        .insert(Collider::rectangle(WINDOW_WIDTH, 1.0))
        .insert(Name::new("Wall_Top"))
        .insert(TransformBundle::from(Transform::from_xyz(
            0.0,
            HALF_HEIGHT,
            0.0,
        )));

    // create the left wall
    commands
        .spawn(RigidBody::Static)
        .insert(Collider::rectangle(1.0, WINDOW_HEIGHT))
        .insert(Name::new("Wall_Left"))
        .insert(TransformBundle::from(Transform::from_xyz(
            -HALF_WIDTH,
            0.0,
            0.0,
        )));

    // create the right wall
    commands
        .spawn(RigidBody::Static)
        .insert(Collider::rectangle(1.0, WINDOW_HEIGHT))
        .insert(Name::new("Wall_Right"))
        .insert(TransformBundle::from(Transform::from_xyz(
            HALF_WIDTH, 0.0, 0.0,
        )));

    // create the bottom
    commands
        .spawn(RigidBody::Static)
        .insert(Collider::rectangle(WINDOW_WIDTH, 1.0))
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
        .spawn(Player)
        .insert(Name::new("Player"))
        .insert(RigidBody::Kinematic)
        .insert(Collider::circle(PLAYER_RADIUS))
        .insert(GravityScale(0.0))
        .insert(Mass(10.0))
        .insert(LockedAxes::new().lock_rotation())
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
        .insert(Xp(0));
}

fn spawn_enemy(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let enemy_shape = Mesh2dHandle(meshes.add(Circle { radius: ENEMY_RADIUS }));
    let enemy_material = materials.add(Color::rgb(255.0, 0.0, 0.0));

    // configure and spawn the enemy
    commands
        .spawn(Enemy)
        .insert(Name::new("Enemy"))
        .insert(RigidBody::Dynamic)
        .insert(Collider::circle(ENEMY_RADIUS))
        .insert(GravityScale(0.0))
        .insert(Mass(1000.0))
        .insert(Restitution::new(0.0))
        .insert(LockedAxes::new().lock_rotation())
        .insert(LinearDamping(0.8))
        .insert(AngularDamping(1.6))
        .insert(TransformBundle::from(Transform {
            translation: ENEMY_POSITION,
            ..default()
        }))
        .insert(MaterialMesh2dBundle {
            mesh: enemy_shape,
            material: enemy_material,
            ..default()
        })
        .insert(InputManagerBundle::with_map(Action::default_input_map()))
        .insert(Health {
            current: 1000,
            max: 1000,
        })
        .insert(Xp(0));
}

fn minion_spawner(
    mut commands: Commands,
    mut er_spawn_minion: EventReader<SpawnMinionEvent>,
    player_pos_query: Query<&Position, With<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let minion_shape = Mesh2dHandle(meshes.add(Circle { radius: 10.0 }));
    let minion_material = materials.add(Color::rgb(0.0, 0.0, 255.0));

    for event in er_spawn_minion.read() {
        let player_pos = player_pos_query.single();

        let gap = 5.0;

        let minion_x = player_pos.x + PLAYER_RADIUS + (gap + MINION_RADIUS) * (event.0 + 1.0);
        let minion_y = player_pos.y + PLAYER_RADIUS + (gap + MINION_RADIUS) * (event.0 + 1.0);

        commands
            .spawn(Minion)
            .insert(Name::new("Minion"))
            .insert(RigidBody::Dynamic)
            .insert(Collider::circle(MINION_RADIUS))
            .insert(GravityScale(0.0))
            .insert(Mass(100.0))
            .insert(LockedAxes::new().lock_rotation())
            .insert(LinearDamping(0.8))
            .insert(AngularDamping(1.6))
            .insert(TransformBundle::from(Transform {
                translation: Vec3::new(minion_x, minion_y, 0.0),
                ..default()
            }));
    }
}

fn handle_actions(
    time: Res<Time>,
    action_query: Query<&ActionState<Action>, With<Player>>,
    mut player_xform_query: Query<&mut Position, With<Player>>,
    mut ew_spawn_minion: EventWriter<SpawnMinionEvent>,
) {
    for action_state in action_query.iter() {
        let mut vel_x = 0.0;
        let mut vel_y = 0.0;

        let speed = PLAYER_SPEED * time.delta_seconds();

        if action_state.pressed(&Action::Up) {
            vel_y = speed;
        }

        if action_state.pressed(&Action::Down) {
            vel_y = -speed;
        }

        if action_state.pressed(&Action::Left) {
            vel_x = -speed;
        }

        if action_state.pressed(&Action::Right) {
            vel_x = speed;
        }

        for mut position in player_xform_query.iter_mut() {
            // clamp x position within the window
            if (position.x + vel_x < HALF_WIDTH - PLAYER_RADIUS)
                && (position.x + vel_x > -HALF_WIDTH + PLAYER_RADIUS) {
                position.x += vel_x;
            }

            // clamp y position within the window
            if (position.y + vel_y < HALF_HEIGHT - PLAYER_RADIUS)
                && (position.y + vel_y > -HALF_HEIGHT + PLAYER_RADIUS) {
                position.y += vel_y;
            }
        }

        if action_state.just_pressed(&Action::Fire) {
            for i in 1..6 {
                ew_spawn_minion.send(SpawnMinionEvent(i as f32));
            }
        }
    }
}

fn enemy_movement(
    time: Res<Time>,
    target_query: Query<&Transform, With<Player>>,
    mut chaser_query: Query<(&Transform, &mut LinearVelocity), With<Enemy>>,
) {
    let pos_target = target_query.single().translation;
    let speed = ENEMY_SPEED * time.delta_seconds();

    for (transform, mut linear_vel) in chaser_query.iter_mut() {
        let pos_chaser = transform.translation;
        let direction = Vec2::normalize(pos_target.xy() - pos_chaser.xy());
        let _distance = pos_target.distance(pos_chaser) - PLAYER_RADIUS - ENEMY_RADIUS;
        linear_vel.x += direction.x * speed;
        linear_vel.y += direction.y * speed;
    }
}

fn minion_movement(
    time: Res<Time>,
    target_query: Query<&Transform, With<Enemy>>,
    mut chaser_query: Query<(&Transform, &mut LinearVelocity), With<Minion>>,
) {
    let pos_target = target_query.single().translation; // TODO: potentially make this a loop and have the minions attack the closest enemy
    let speed = PLAYER_SPEED * time.delta_seconds();

    for (transform, mut linear_vel) in chaser_query.iter_mut() {
        let pos_chaser = transform.translation;
        let direction = Vec2::normalize(pos_target.xy() - pos_chaser.xy());
        let _distance = pos_target.distance(pos_chaser) - MINION_RADIUS - ENEMY_RADIUS;
        linear_vel.x += direction.x * speed;
        linear_vel.y += direction.y * speed;
    }
}
