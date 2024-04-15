use bevy::input::common_conditions::input_toggle_active;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::{EnabledButtons, ExitCondition, PresentMode, WindowResolution};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_xpbd_2d::prelude::*;
use bevy_screen_diagnostics::{ScreenDiagnosticsPlugin, ScreenFrameDiagnosticsPlugin};
use bevy_xpbd_2d::math::Vector;
use leafwing_input_manager::plugin::InputManagerPlugin;
use leafwing_input_manager::prelude::*;
const WINDOW_WIDTH: f32 = 768.0;
const WINDOW_HEIGHT: f32 = 512.0;

const HALF_HEIGHT: f32 = WINDOW_HEIGHT / 2.0;
const HALF_WIDTH: f32 = WINDOW_WIDTH / 2.0;

const PLAYER_SPEED: f32 = 400.0;
const PLAYER_RADIUS: f32 = 25.0;
const PLAYER_POSITION: Vector = Vector::new(
    -HALF_WIDTH + PLAYER_RADIUS + 5.0,
    -HALF_HEIGHT + PLAYER_RADIUS + 5.0,
);

const ENEMY_SPEED: f32 = 800.0;
const ENEMY_RADIUS: f32 = PLAYER_RADIUS * 1.25;
const ENEMY_POSITION: Vector = Vector::new(
    HALF_WIDTH - ENEMY_RADIUS - 5.0,
    HALF_HEIGHT - ENEMY_RADIUS - 5.0,
);

const MINION_SPEED: f32 = ENEMY_SPEED * 2.0;
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
                title: "LD55 - Bomb the slime to survive! (Theme: Summoning)".to_string(),
                ..default()
            }),
            exit_condition: ExitCondition::OnPrimaryClosed,
            ..default()
        }).set(LogPlugin {
            filter: "info,wgpu=error,ld55_summoning=info".into(),
            level: bevy::log::Level::DEBUG,
            ..default()
        }),)
        .add_plugins(
            WorldInspectorPlugin::default().run_if(input_toggle_active(false, KeyCode::F1)),
        )
        .add_plugins(ScreenDiagnosticsPlugin::default())
        .add_plugins(ScreenFrameDiagnosticsPlugin)
        .add_plugins(InputManagerPlugin::<PlayerAction>::default())
        .add_plugins(PhysicsPlugins::default())
        // .add_plugins(PhysicsDebugPlugin::default())
        // events
        .add_event::<SpawnMinionEvent>()
        .add_event::<DamageTakenEvent>()
        // systems
        .add_systems(Startup, setup)
        .add_systems(Startup, spawn_enemy.after(setup))
        .add_systems(Update, bevy::window::close_on_esc)
        .add_systems(Update, minion_spawner)
        .add_systems(Update, handle_actions)
        // .add_systems(Update, handle_actions_touch)
        // .add_systems(Update, touch_resource)
        .add_systems(Update, enemy_movement)
        .add_systems(Update, minion_movement)
        .add_systems(Update, handle_collisions)
        .add_systems(Update, handle_damage_taken)
        .add_systems(Update, update_health_bars)
        // .add_systems(Update, dev_tools_system)
        // resources
        .insert_resource(SubstepCount(6))
        // start
        .run();
}

#[derive(Component)]
struct CameraMarker;

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
struct DamageDone(i32);

#[derive(Event, Debug)]
struct SpawnMinionEvent(f32);

#[derive(Event, Debug)]
struct DamageTakenEvent {
    entity: Entity,
    damage: i32,
}

#[derive(Component, Debug)]
struct HealthBar;

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
enum PlayerAction {
    Move,
    SpawnMinions,
}

impl PlayerAction {
    fn default_input_map() -> InputMap<Self> {
        let mut input_map = InputMap::default();

        // keyboard
        input_map.insert(Self::Move, VirtualDPad::wasd());
        input_map.insert(Self::Move, VirtualDPad::arrow_keys());
        input_map.insert(Self::SpawnMinions, KeyCode::Space);

        // gamepad
        input_map.insert(Self::Move, DualAxis::left_stick());
        input_map.insert(Self::SpawnMinions, GamepadButtonType::South);

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
    commands.spawn(Camera2dBundle::default())
        .insert(CameraMarker);

    // spawn some instructions
    let font = asset_server.load("fonts/FiraSansExtraCondensed-Regular.ttf");
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                "Move: WASD/Arrows/Left Stick | Spawn Minions: Space Bar/Gamepad A",
                TextStyle {
                    font: font.clone(),
                    font_size: 24.0,
                    color: Color::WHITE,
                }),
            text_anchor: Anchor::TopLeft,
            transform: Transform {
                translation: Vec3::new(-HALF_WIDTH, HALF_HEIGHT, 0.0),
                ..default()
            },
            ..default()
        },
    ));

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
        .insert(Restitution::new(0.0))
        .insert(Position(PLAYER_POSITION))
        .insert(CollisionLayers::new(GameLayer::Player, [GameLayer::Enemy]))
        .insert(SpriteBundle {
            texture: asset_server.load("Sprite-Player.png"),
            ..default()
        })
        .insert(InputManagerBundle::with_map(PlayerAction::default_input_map()))
        .insert(Health{
            current: 10,
            max: 10,
        })
        .insert(DamageDone(0));
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
        .insert(LinearDamping(0.8))
        .insert(AngularDamping(1.6))
        .insert(CollisionLayers::new(GameLayer::Enemy, [GameLayer::Player, GameLayer::Minion]))
        .insert(Position(ENEMY_POSITION))
        .insert(SpriteBundle {
            texture: asset_server.load("Sprite-Enemy.png"),
            ..default()
        })
        .insert(Health{
            current: 500,
            max: 500,
        })
        .insert(DamageDone(15))
        .with_children(|parent| {
            parent.spawn((Text2dBundle {
                text: Text::from_section(
                    "HP: ",
                    TextStyle {
                        font: asset_server.load("fonts/FiraSansCondensed-Regular.ttf"),
                        font_size: 24.0,
                        color: Color::WHITE,
                    }),
                text_anchor: Anchor::BottomCenter,
                transform: Transform {
                    translation: Vec3::new(0.0, ENEMY_RADIUS + 2.0, 0.0),
                    rotation: Quat::default(),
                    ..default()
                },
                ..default()
            }, HealthBar));
        });
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
        let minion_pos = Vector::new(
            player_pos.x + PLAYER_RADIUS + (gap + MINION_RADIUS) * (event.0 + 1.0),
            player_pos.y + PLAYER_RADIUS + (gap + MINION_RADIUS) * (event.0 + 1.0),
        );

        debug!("Spawning new minion (#{}) at {}.", event.0, player_pos);

        commands
            .spawn(Minion)
            .insert(Name::new("Minion"))
            .insert(RigidBody::Dynamic)
            .insert(Collider::circle(MINION_RADIUS))
            .insert(GravityScale(0.0))
            .insert(Mass(50.0))
            .insert(Restitution::new(1.0))
            .insert(LinearDamping(0.8))
            .insert(AngularDamping(1.6))
            .insert(CollisionLayers::new(GameLayer::Minion, [GameLayer::Minion, GameLayer::Enemy]))
            .insert(Position(minion_pos))
            .insert(SpriteBundle {
                texture: asset_server.load("Sprite-Bomb.png"),
                ..default()
            })
            .insert(Health{
                current: 20,
                max: 20,
            })
            .insert(DamageDone(15))
            .with_children(|parent| {
                parent.spawn((Text2dBundle {
                    text: Text::from_section(
                        "HP: ",
                        TextStyle {
                            font: asset_server.load("fonts/FiraSansCondensed-Regular.ttf"),
                            font_size: 18.0,
                            color: Color::WHITE,
                        }),
                    text_anchor: Anchor::BottomCenter,
                    transform: Transform {
                        translation: Vec3::new(0.0, MINION_RADIUS + 2.0, 0.0),
                        rotation: Quat::default(),
                        ..default()
                    },
                    ..default()
                }, HealthBar));
            });
    }
}

fn handle_actions(
    time: Res<Time>,
    action_query: Query<&ActionState<PlayerAction>, With<Player>>,
    mut player_xform_query: Query<&mut Position, With<Player>>,
    mut ew_spawn_minion: EventWriter<SpawnMinionEvent>,
) {
    for action_state in action_query.iter() {
        let speed = PLAYER_SPEED * time.delta_seconds();

        if action_state.pressed(&PlayerAction::Move) {
            let move_delta = speed
                * action_state
                .clamped_axis_pair(&PlayerAction::Move)
                .unwrap()
                .xy();

            if let Ok(mut position) = player_xform_query.get_single_mut() {
                // clamp x position within the window
                if (position.x + move_delta.x < HALF_WIDTH - PLAYER_RADIUS)
                    && (position.x + move_delta.x > -HALF_WIDTH + PLAYER_RADIUS) {
                    position.x += move_delta.x;
                }

                // clamp y position within the window
                if (position.y + move_delta.y < HALF_HEIGHT - PLAYER_RADIUS)
                    && (position.y + move_delta.y > -HALF_HEIGHT + PLAYER_RADIUS) {
                    position.y += move_delta.y;
                }
            }
        }

        if action_state.just_pressed(&PlayerAction::SpawnMinions) {
            for i in 1..6 {
                ew_spawn_minion.send(SpawnMinionEvent(i as f32));
            }
        }
    }
}

// fn touch_resource(
//     touches: Res<Touches>,
//     mut ew_spawn_minion: EventWriter<SpawnMinionEvent>,
// ) {
//     for finger in touches.iter() {
//         if touches.just_pressed(finger.id()) {
//             for i in 1..6 {
//                 ew_spawn_minion.send(SpawnMinionEvent(i as f32));
//             }
//         }
//     }
// }

// fn handle_actions_touch(
//     mut er_touch: EventReader<TouchInput>,
//     mut ew_spawn_minion: EventWriter<SpawnMinionEvent>,
// ) {
//     for event in er_touch.read() {
//         for i in 1..6 {
//             ew_spawn_minion.send(SpawnMinionEvent(i as f32));
//         }
//
//         // match event.phase {
//         //     TouchPhase::Ended => {
//         //         for i in 1..6 {
//         //             ew_spawn_minion.send(SpawnMinionEvent(i as f32));
//         //         }
//         //     }
//         //     _ => ()
//         // }
//     }
// }

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
        let speed = MINION_SPEED * time.delta_seconds();

        for (transform, mut linear_vel) in chaser_query.iter_mut() {
            let pos_chaser = transform.translation;
            let direction = Vec2::normalize(pos_target.xy() - pos_chaser.xy());
            linear_vel.x += direction.x * speed;
            linear_vel.y += direction.y * speed;
        }
    }
}

fn handle_collisions(
    mut event_reader_collisions: EventReader<CollisionStarted>,
    minion_query: Query<&Minion>,
    damage_done_query: Query<&DamageDone>,
    mut ew_damage_taken: EventWriter<DamageTakenEvent>,
) {
    for CollisionStarted(entity1, entity2) in event_reader_collisions.read() {
        if let Ok(damage) = damage_done_query.get(*entity1) {
            // ignore minion-minion collisions
            if let Ok(_entity) = minion_query.get(*entity1) { // TODO: there has to be a better way of doing this
                if let Ok(_entity) = minion_query.get(*entity2) {
                    trace!("ignoring minion-minion collisions");
                    continue;
                }
            }

            ew_damage_taken.send(DamageTakenEvent {
                entity: entity2.clone(), // TODO: handle the lifetime properly
                damage: damage.0,
            });
        }
    }
}

fn handle_damage_taken(
    mut commands: Commands,
    mut er_damage_taken: EventReader<DamageTakenEvent>,
    mut health_query: Query<(&mut Health, &Name), With<Health>>,
) {
    for event in er_damage_taken.read() {
        if let Ok((mut health,name)) = health_query.get_mut(event.entity) {
            health.current -= event.damage;
            debug!(
                "{} ({:?}) takes {:?} damage (final health = {:?})",
                name,
                event.entity,
                event.damage,
                health.current,
            );

            if health.current <= 0 {
                debug!("{} ({:?}) dies.", name, event.entity);
                commands.entity(event.entity).despawn_recursive();
            }
        }
    }
}

fn update_health_bars(
    mut health_bar_query: Query<&mut Text, With<HealthBar>>,
    health_query: Query<(&Health, &Children), With<Health>>,
) {
    for (health, children) in health_query.iter() {
        for child in children.iter() {
            if let Ok(mut text) = health_bar_query.get_mut(*child) {
                text.sections[0].value = format!("{}", health.current);
            }
        }
    }
}

