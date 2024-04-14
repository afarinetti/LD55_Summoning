use bevy::input::common_conditions::input_toggle_active;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::window::{EnabledButtons, ExitCondition, PresentMode, WindowResolution};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_xpbd_2d::prelude::*;
use bevy_screen_diagnostics::{ScreenDiagnosticsPlugin, ScreenFrameDiagnosticsPlugin};
use bevy_xpbd_2d::math::Vector;
use leafwing_input_manager::plugin::InputManagerPlugin;
use leafwing_input_manager::prelude::*;

const WINDOW_WIDTH: f32 = 1024.0;
const WINDOW_HEIGHT: f32 = 768.0;

const HALF_HEIGHT: f32 = WINDOW_HEIGHT / 2.0;
const HALF_WIDTH: f32 = WINDOW_WIDTH / 2.0;

const PLAYER_SPEED: f32 = 400.0;
const PLAYER_RADIUS: f32 = 25.0;
const PLAYER_POSITION: Vector = Vector::new(
    -HALF_WIDTH + PLAYER_RADIUS + 5.0,
    -HALF_HEIGHT + PLAYER_RADIUS + 5.0,
);

const ENEMY_SPEED: f32 = PLAYER_SPEED * 0.5;
const ENEMY_RADIUS: f32 = PLAYER_RADIUS * 1.5;
const ENEMY_POSITION: Vector = Vector::new(
    HALF_WIDTH - ENEMY_RADIUS - 5.0,
    HALF_HEIGHT - ENEMY_RADIUS - 5.0,
);

const MINION_SPEED: f32 = PLAYER_SPEED * 1.5;
const MINION_RADIUS: f32 = PLAYER_RADIUS / 2.0;


fn main() {
    // determine window the present mode based on compilation target
    let present_mode: PresentMode = if cfg!(target_family = "wasm") {
        PresentMode::Fifo       // required for wasm builds
    } else {
        PresentMode::Immediate  // needed on some linux distros
    };
    
    App::new()
        // plugins
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode,
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
        .add_event::<DamageTakenEvent>()
        // systems
        .add_systems(Startup, setup)
        .add_systems(Startup, spawn_enemy.after(setup))
        .add_systems(Update, bevy::window::close_on_esc)
        .add_systems(Update, minion_spawner)
        .add_systems(Update, handle_actions)
        .add_systems(Update, enemy_movement)
        .add_systems(Update, minion_movement)
        .add_systems(Update, handle_collisions)
        .add_systems(Update, handle_damage_taken)
        // .add_systems(Update, dev_tools_system)
        // resources
        .insert_resource(SubstepCount(6))
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

#[derive(Component, Debug)]
struct Player;

#[derive(Component, Debug)]
struct Minion;

#[derive(Component, Debug)]
struct Enemy;

#[derive(Component, Debug)]
struct Health {
    current: i32,
    max: i32,
}

#[derive(Component, Debug, Copy, Clone)]
struct Damage(i32);

#[derive(Component, Debug)]
struct Xp(u32);

#[derive(Event, Debug)]
struct SpawnMinionEvent(f32);

#[derive(Event, Debug)]
struct DamageTakenEvent {
    damaged_entity: Entity,
    damage: i32,
}

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

#[derive(PhysicsLayer)]
enum GameLayer {
    Player, // Layer 0
    Minion, // Layer 1
    Enemy,  // Layer 3
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>
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

    // configure and spawn the player
    commands
        .spawn(Player)
        .insert(Name::new("Player"))
        .insert(RigidBody::Kinematic)
        .insert(Collider::circle(PLAYER_RADIUS))
        .insert(GravityScale(0.0))
        .insert(Mass(10.0))
        // .insert(LockedAxes::new().lock_rotation())
        .insert(Position(PLAYER_POSITION))
        .insert(CollisionLayers::new(GameLayer::Player, [GameLayer::Enemy]))
        .insert(SpriteBundle {
            texture: asset_server.load("Sprite-Stomach.png"),
            ..default()
        })
        .insert(InputManagerBundle::with_map(Action::default_input_map()))
        .insert(Health {
            current: 10,
            max: 10,
        })
        .insert(Damage(0))
        .insert(Xp(0));
}

fn spawn_enemy(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // configure and spawn the enemy
    commands
        .spawn(Enemy)
        .insert(Name::new("Enemy"))
        .insert(RigidBody::Dynamic)
        .insert(Collider::circle(ENEMY_RADIUS))
        .insert(GravityScale(0.0))
        .insert(Mass(1000.0))
        .insert(Restitution::new(0.0))
        // .insert(LockedAxes::new().lock_rotation())
        .insert(LinearDamping(0.8))
        .insert(AngularDamping(1.6))
        .insert(CollisionLayers::new(GameLayer::Enemy, [GameLayer::Player, GameLayer::Minion]))
        .insert(Position(ENEMY_POSITION))
        // .insert(TransformBundle::from(Transform {
        //     translation: ENEMY_POSITION,
        //     ..default()
        // }))
        .insert(SpriteBundle {
            texture: asset_server.load("Sprite-IceCreamBoss.png"),
            ..default()
        })
        .insert(Health {
            current: 1000,
            max: 1000,
        })
        .insert(Damage(10))
        .insert(Xp(0));
}

fn minion_spawner(
    mut commands: Commands,
    mut er_spawn_minion: EventReader<SpawnMinionEvent>,
    player_pos_query: Query<&Transform, With<Player>>,
    asset_server: Res<AssetServer>,
) {
    for event in er_spawn_minion.read() {
        let player_pos = player_pos_query.single().translation;

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
            // .insert(LockedAxes::new().lock_rotation())
            .insert(LinearDamping(0.8))
            .insert(AngularDamping(1.6))
            .insert(CollisionLayers::new(GameLayer::Minion, [GameLayer::Minion, GameLayer::Enemy]))
            .insert(TransformBundle::from(Transform {
                translation: Vec3::new(minion_x, minion_y, 0.0),
                ..default()
            }))
            .insert(SpriteBundle {
                texture: asset_server.load("Sprite-Lactaid.png"),
                ..default()
            })
            .insert(Health {
                current: 100,
                max: 100,
            })
            .insert(Damage(10));
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
    if let Ok(pos_xform) = target_query.get_single() {
        let pos_target = pos_xform.translation;
        let speed = ENEMY_SPEED * time.delta_seconds();

        for (transform, mut linear_vel) in chaser_query.iter_mut() {
            let pos_chaser = transform.translation;
            let direction = Vec2::normalize(pos_target.xy() - pos_chaser.xy());
            let _distance = pos_target.distance(pos_chaser) - PLAYER_RADIUS - ENEMY_RADIUS;
            linear_vel.x += direction.x * speed;
            linear_vel.y += direction.y * speed;
        }
    }
}

fn minion_movement(
    time: Res<Time>,
    target_query: Query<&Transform, With<Enemy>>,
    mut chaser_query: Query<(&Transform, &mut LinearVelocity), With<Minion>>,
) {
    if let Ok(pos_xform) = target_query.get_single() {
        let pos_target = pos_xform.translation;
        let speed = PLAYER_SPEED * time.delta_seconds();

        for (transform, mut linear_vel) in chaser_query.iter_mut() {
            let pos_chaser = transform.translation;
            let direction = Vec2::normalize(pos_target.xy() - pos_chaser.xy());
            let _distance = pos_target.distance(pos_chaser) - MINION_RADIUS - ENEMY_RADIUS;
            linear_vel.x += direction.x * speed;
            linear_vel.y += direction.y * speed;
        }
    }
}

fn handle_collisions(
    enemy_collision_query: Query<(Entity, &Damage, &CollidingEntities), With<Enemy>>,
    minion_collision_query: Query<(Entity, &Damage, &CollidingEntities), With<Minion>>,
    minion_query: Query<Entity, With<Minion>>,
    mut ew_damage_taken: EventWriter<DamageTakenEvent>,
) {
    // boss collisions
    for (e_entity, damage, colliding_entities) in &enemy_collision_query {
        if !colliding_entities.is_empty() {
            trace!("BOSS ({:?}): {:?} --> {:?}", e_entity, damage, colliding_entities);
            for colliding_entity in colliding_entities.iter() {
                ew_damage_taken.send(DamageTakenEvent {
                    damaged_entity: colliding_entity.clone(), // TODO: handle the lifetime properly
                    damage: damage.0,
                });
            }
        }
    }

    // minion collisions
    for (m_entity, damage, colliding_entities) in &minion_collision_query {
        if !colliding_entities.is_empty() {
            trace!("MINION ({:?}): {:?} --> {:?}", m_entity, damage, colliding_entities);
            for colliding_entity in colliding_entities.iter() {
                if let Ok(_entity) = minion_query.get(*colliding_entity) {
                    trace!("ignoring minion-minion collision.")
                } else {
                    ew_damage_taken.send(DamageTakenEvent {
                        damaged_entity: colliding_entity.clone(), // TODO: handle the lifetime properly
                        damage: damage.0,
                    });
                }
            }
        }
    }
}

fn handle_damage_taken(
    mut commands: Commands,
    mut er_damage_taken: EventReader<DamageTakenEvent>,
    mut health_query: Query<&mut Health, With<Health>>,
) {
    for event in er_damage_taken.read() {
        if let Ok(mut health) = health_query.get_mut(event.damaged_entity) {
            health.current -= event.damage;
            info!(
                "Entity {:?} takes {:?} damage (final health = {:?})",
                event.damaged_entity,
                event.damage,
                health.current,
            );
            if health.current <= 0 {
                info!(
                    "Entity {:?} dies.",
                    event.damaged_entity,
                );
                commands.entity(event.damaged_entity).despawn();
            }
        }
    }
}

