use std::time::Duration;
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
use rand::Rng;

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
        .add_event::<ManaGainedEvent>()
        // systems
        .add_systems(Startup, setup)
        .add_systems(Startup, spawn_player.after(setup))
        .add_systems(Startup, spawn_enemy.after(spawn_player))
        .add_systems(Startup, setup_mana_spawning)
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
        .add_systems(Update, update_mana_bar)
        .add_systems(Update, mana_spawner)
        .add_systems(Update, handle_mana_gained)
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

#[derive(Component, Debug)]
struct Mana {
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
    amount: i32,
}

#[derive(Component, Debug)]
struct HealthBar;

#[derive(Component, Debug)]
struct ManaBar;

#[derive(Component, Debug)]
struct ManaGem(i32);

#[derive(Resource, Debug)]
struct ManaSpawnConfig {
    timer: Timer,
}

#[derive(Event, Debug)]
struct ManaGainedEvent {
    player: Entity,
    mana_gem: Entity,
    amount: i32,
}

#[derive(Resource, Debug)]
struct FontResource {
    font: Handle<Font>,
}

#[derive(Resource, Debug)]
struct SpriteResource {
    player: Handle<Image>,
    enemy: Handle<Image>,
    minion: Handle<Image>,
    mana_gem: Handle<Image>,
}

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
    Gems,   // Layer 4
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>
) {
    // configure and spawn theS camera
    commands.spawn(Camera2dBundle::default())
        .insert(CameraMarker);

    // load and tile background image
    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("images/Background-Rock.png"),
            sprite: Sprite {
              custom_size: Some(Vec2::new(WINDOW_WIDTH, WINDOW_WIDTH)),
              ..default()
            },
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, -1.0),
                ..default()
            },
            ..default()
        },
        ImageScaleMode::Tiled {
            tile_x: true,
            tile_y: true,
            stretch_value: 1.0,
        },
    ));

    // load font(s)
    let font_handle = asset_server.load("fonts/FiraSansCondensed-Regular.ttf");
    commands.insert_resource(FontResource {
        font: font_handle.clone(),
    });

    // spawn some instructions
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                "Move: WASD/Arrows/Left Stick | Spawn Bombs: Space Bar/Gamepad A",
                TextStyle {
                    font: font_handle.clone(),
                    font_size: 20.0,
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
    commands.spawn(( // TODO: make this a section
        Text2dBundle {
            text: Text::from_section(
                "Don't get hit. Spawn bombs, collect mana gems. Survive.",
                TextStyle {
                    font: font_handle.clone(),
                    font_size: 20.0,
                    color: Color::WHITE,
                }),
            text_anchor: Anchor::TopLeft,
            transform: Transform {
                translation: Vec3::new(-HALF_WIDTH, HALF_HEIGHT - 20.0, 0.0),
                ..default()
            },
            ..default()
        },
    ));

    // spawn the player's mana bar
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                "MP: ",
                TextStyle {
                    font: font_handle.clone(),
                    font_size: 24.0,
                    color: Color::ALICE_BLUE,
                }),
            text_anchor: Anchor::BottomLeft,
            transform: Transform {
                translation: Vec3::new(-HALF_WIDTH, -HALF_HEIGHT, 0.0),
                ..default()
            },
            ..default()
        },
        ManaBar
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

    // pre-load sprites
    commands.insert_resource(SpriteResource {
        player: asset_server.load("images/Sprite-Player.png"),
        enemy: asset_server.load("images/Sprite-Enemy.png"),
        minion: asset_server.load("images/Sprite-Bomb.png"),
        mana_gem: asset_server.load("images/Sprite-ManaGem.png"),
    });
}

fn spawn_player(
    mut commands: Commands,
    sprite_res: Res<SpriteResource>,
) {
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
        .insert(CollisionLayers::new(GameLayer::Player, [GameLayer::Enemy, GameLayer::Gems]))
        .insert(SpriteBundle {
            texture: sprite_res.player.clone(),
            ..default()
        })
        .insert(InputManagerBundle::with_map(PlayerAction::default_input_map()))
        .insert(Health {
            current: 10,
            max: 10,
        })
        .insert(Mana {
            current: 50,
            max: 50,
        })
        .insert(DamageDone(0));
}

fn spawn_enemy(
    mut commands: Commands,
    sprite_res: Res<SpriteResource>,
    font_res: Res<FontResource>,
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
            texture: sprite_res.enemy.clone(),
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
                        font: font_res.font.clone(),
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
    sprite_res: Res<SpriteResource>,
    font_res: Res<FontResource>,
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
                texture: sprite_res.minion.clone(),
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
                            font: font_res.font.clone(),
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
    mut player_mana_query: Query<&mut Mana, With<Player>>,
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
            let mana_cost = 10;
            if let Ok(mut mana) = player_mana_query.get_single_mut() { // TODO: move this logic to the minion spawner
                if mana.current >= mana_cost {
                    mana.current -= mana_cost;
                    for i in 1..2 {
                        ew_spawn_minion.send(SpawnMinionEvent(i as f32));
                    }
                }
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
    mana_gem_query: Query<&ManaGem>,
    mut ew_damage_taken: EventWriter<DamageTakenEvent>,
    mut ew_mana_gained: EventWriter<ManaGainedEvent>,
) {
    for CollisionStarted(entity1, entity2) in event_reader_collisions.read() {
        // damaging collisions
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
                amount: damage.0,
            });
        }

        // mana gem collisions
        if let Ok(mana_gem) = mana_gem_query.get(*entity2) {
            ew_mana_gained.send(ManaGainedEvent {
                player: entity1.clone(), // TODO: handle the lifetime properly
                mana_gem: entity2.clone(), // TODO: handle the lifetime properly
                amount: mana_gem.0,
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
            health.current = std::cmp::max(0, health.current - event.amount);

            debug!(
                "{} ({:?}) takes {:?} damage (final health = {:?})",
                name,
                event.entity,
                event.amount,
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

fn update_mana_bar(
    mut mana_bar_query: Query<&mut Text, With<ManaBar>>,
    mana_query: Query<&Mana>,
) {
    for mana in mana_query.iter() {
        if let Ok(mut text) = mana_bar_query.get_single_mut() {
            text.sections[0].value = format!("MP: {:3}/{:3}", mana.current, mana.max);
        }
    }
}

fn setup_mana_spawning(
    mut commands: Commands,
) {
    commands.insert_resource(ManaSpawnConfig {
        timer: Timer::new(Duration::from_secs(3), TimerMode::Repeating),
    })
}

fn mana_spawner(
    mut commands: Commands,
    sprite_res: Res<SpriteResource>,
    time: Res<Time>,
    mut config: ResMut<ManaSpawnConfig>,
    mana_gem_query: Query<&ManaGem>,
) {
    // tick the timer
    config.timer.tick(time.delta());

    // if the timer has elapsed, spawn a gem
    if config.timer.finished() && mana_gem_query.iter().len() <= 10 {
        let mut rng = rand::thread_rng();

        let gem_x = rng.gen_range(-HALF_WIDTH as i32..=HALF_WIDTH as i32) as f32;
        let gem_y = rng.gen_range(-HALF_HEIGHT as i32..=HALF_HEIGHT as i32) as f32;
        let gem_pos = Vector::new(gem_x, gem_y);

        debug!("Spawning new mana gem at {}.", gem_pos);

        commands
            .spawn(ManaGem(10))
            .insert(Name::new("ManaGem"))
            .insert(RigidBody::Kinematic)
            .insert(Collider::circle(10.0))
            .insert(CollisionLayers::new(GameLayer::Gems, [GameLayer::Player]))
            .insert(Position(gem_pos))
            .insert(SpriteBundle {
                texture: sprite_res.mana_gem.clone(),
                ..default()
            });
    }
}

fn handle_mana_gained(
    mut commands: Commands,
    mut er_mana_gained: EventReader<ManaGainedEvent>,
    mut mana_query: Query<(&mut Mana, &Name), With<Mana>>,
) {
    for event in er_mana_gained.read() {
        if let Ok((mut mana, name)) = mana_query.get_mut(event.player) {
            if mana.current < mana.max {
                mana.current = std::cmp::min(mana.max, mana.current + event.amount);

                debug!(
                    "{} ({:?}) gains {:?} mana (final mana total = {:?})",
                    name,
                    event.player,
                    event.amount,
                    mana.current,
                );

                commands.entity(event.mana_gem).despawn();
            }
        }
    }
}
