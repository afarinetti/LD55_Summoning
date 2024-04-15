mod classes;
mod audio;
mod loading;

use bevy::input::common_conditions::input_toggle_active;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::{EnabledButtons, ExitCondition, PresentMode, WindowResolution};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_screen_diagnostics::{ScreenDiagnosticsPlugin, ScreenFrameDiagnosticsPlugin};
use bevy_xpbd_2d::math::Vector;
use bevy_xpbd_2d::prelude::*;
use leafwing_input_manager::plugin::InputManagerPlugin;
use leafwing_input_manager::prelude::*;
use rand::Rng;
use std::cmp;
use std::time::Duration;
use bevy_ui_dsl::*;
use classes::*;
use bevy_asset_loader::prelude::*;
use bevy_kira_audio::{Audio, AudioApp, AudioChannel, AudioControl, AudioInstance, AudioPlugin};
use audio::*;
use loading::*;

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
const MINION_RADIUS: f32 = (PLAYER_RADIUS / 2.0) + 5.0;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum GameState {
    #[default]
    AssetLoading,
    MainMenu,
    InGame,
    GameOver,
}

fn main() {
    // determine window the present mode based on compilation target
    let present_mode: PresentMode = if cfg!(target_family = "wasm") {
        PresentMode::Fifo // required for wasm builds
    } else {
        PresentMode::Immediate // needed on some linux distros
    };

    App::new()
        // plugins
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
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
                })
                .set(LogPlugin {
                    filter: "info,wgpu=error,ld55_summoning=info".into(),
                    level: bevy::log::Level::DEBUG,
                    ..default()
                }),
        )
        .add_plugins(
            WorldInspectorPlugin::default().run_if(input_toggle_active(false, KeyCode::F1)),
        )
        .add_plugins(ScreenDiagnosticsPlugin::default())
        .add_plugins(ScreenFrameDiagnosticsPlugin)
        .add_plugins(InputManagerPlugin::<PlayerAction>::default())
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(AudioPlugin)
        // .add_audio_channel::<MusicChannel>()
        // .add_audio_channel::<EffectsChannel>()
        // .add_plugins(PhysicsDebugPlugin::default())
        .add_loading_state(
            LoadingState::new(GameState::AssetLoading)
                .continue_to_state(GameState::MainMenu)
                .load_collection::<AudioAssets>()
                .load_collection::<SpriteAssets>()
        )

        // events
        .add_event::<SpawnMinionEvent>()
        .add_event::<DamageTakenEvent>()
        .add_event::<ManaGainedEvent>()

        // states
        .init_state::<GameState>()

        // pre-startup systems
        .add_systems(Startup, pre_startup_init)

        // on-enter: main menu
        .add_systems(OnEnter(GameState::MainMenu), (
            setup_main_menu,
            play_bgm,
        ))

        // on-enter: in game
        .add_systems(OnEnter(GameState::InGame), (
            setup_game,
            spawn_player.after(setup_game),
            spawn_enemy.after(spawn_player),
            setup_mana_spawning,
        ))

        // on-enter: game over
        .add_systems(OnEnter(GameState::GameOver), (
            setup_game_over,
        ))

        // update systems
        .add_systems(Update, (
            // main menu
            (
                bevy::window::close_on_esc,
                handle_main_menu_actions,
            ).run_if(in_state(GameState::MainMenu)),
            // in game
            (
                bevy::window::close_on_esc,
                minion_spawner,
                handle_actions,
                enemy_movement,
                minion_movement,
                handle_collisions,
                handle_damage_taken.after(handle_collisions),
                update_health_bars.after(handle_damage_taken),
                handle_mana_gained.after(handle_collisions),
                update_mana_bar.after(handle_mana_gained),
                mana_spawner,
            ).run_if(in_state(GameState::InGame)),
            // game over
            (
                bevy::window::close_on_esc,
                handle_game_over_actions,
            ).run_if(in_state(GameState::GameOver)),
        ))

        // on exit: main menu
        .add_systems(OnExit(GameState::MainMenu), (
            cleanup_main_menu,
        ))

        // on exit: in game
        .add_systems(OnExit(GameState::InGame), (
            cleanup_in_game_screen,
        ))

        // on exit: game over
        .add_systems(OnExit(GameState::GameOver), (
            cleanup_game_over_screen,
        ))

        // resources
        .insert_resource(GameStatus {
            result: GameResult::None,
        })

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
    giver: Entity,
    receiver: Entity,
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

#[derive(Component, PartialEq, Eq, Hash)]
enum MainMenuScreen {
    Node,
    Text,
    BeginButton,
}

#[derive(Component)]
struct InGameScreen;

#[derive(Component, PartialEq, Eq, Hash)]
enum GameOverScreen {
    Node,
    Text,
    RestartButton,
}

#[derive(Debug)]
enum GameResult {
    None,
    Win,
    Lose,
}

#[derive(Resource)]
struct GameStatus {
    result: GameResult,
}

fn pre_startup_init(mut commands: Commands, asset_server: Res<AssetServer>) {
    // configure and spawn the camera
    commands.spawn(Camera2dBundle::default());

    // load font(s)
    let font_handle = asset_server.load("fonts/FiraSansCondensed-Regular.ttf");
    commands.insert_resource(FontResource {
        font: font_handle.clone(),
    });

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
}

fn setup_main_menu(
    mut commands: Commands,
    assets: Res<AssetServer>,
) {
    root(c_root, &assets, &mut commands, |p| {
        nodei(c_no_bg, MainMenuScreen::Node, p, |p| {
            texti("Bomb the slimes to survive!", c_text, c_pixel_title, MainMenuScreen::Text, p);
        });
        nodei(c_no_bg, MainMenuScreen::Node, p, |p| {
            text_buttoni("Begin", c_button, c_pixel_button, MainMenuScreen::BeginButton, p);
        });
    });
}

fn handle_main_menu_actions(
    ui_entities: Query<(&MainMenuScreen, &Interaction), Changed<Interaction>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (id, inter) in &ui_entities {
        if *id == MainMenuScreen::BeginButton && *inter == Interaction::Pressed {
            next_state.set(GameState::InGame);
            break;
        }
    }
}

fn cleanup_main_menu(
    mut commands: Commands,
    query: Query<Entity, With<MainMenuScreen>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn cleanup_in_game_screen(
    mut commands: Commands,
    query: Query<Entity, With<InGameScreen>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn setup_game_over(
    mut commands: Commands,
    assets: Res<AssetServer>,
    game_status: ResMut<GameStatus>,
) {
    root(c_root, &assets, &mut commands, |p| {
        nodei(c_no_bg, GameOverScreen::Node, p, |p| {
            texti(format!("Game over! You {:?}!", game_status.result), c_text, c_pixel_title, GameOverScreen::Text, p);
        });
        nodei(c_no_bg, GameOverScreen::Node, p, |p| {
            text_buttoni("Restart", c_button, c_pixel_button, GameOverScreen::RestartButton, p);
        });
    });
}

fn handle_game_over_actions(
    ui_entities: Query<(&GameOverScreen, &Interaction), Changed<Interaction>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (id, inter) in &ui_entities {
        if *id == GameOverScreen::RestartButton && *inter == Interaction::Pressed {
            next_state.set(GameState::InGame);
            break;
        }
    }
}

fn cleanup_game_over_screen(
    mut commands: Commands,
    query: Query<Entity, With<GameOverScreen>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn setup_game(
    mut commands: Commands,
    font_res: Res<FontResource>,
) {
    // spawn some instructions
    commands.spawn((Text2dBundle {
        text: Text::from_section(
            "Move: WASD/Arrows/Left Stick | Spawn Bombs: Space Bar/Gamepad A",
            TextStyle {
                font: font_res.font.clone(),
                font_size: 20.0,
                color: Color::WHITE,
            },
        ),
        text_anchor: Anchor::TopLeft,
        transform: Transform {
            translation: Vec3::new(-HALF_WIDTH, HALF_HEIGHT, 0.0),
            ..default()
        },
        ..default()
    },InGameScreen));
    commands.spawn((
        // TODO: make this a section
        Text2dBundle {
            text: Text::from_section(
                "Don't get hit. Spawn bombs, collect mana gems. Survive.",
                TextStyle {
                    font: font_res.font.clone(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ),
            text_anchor: Anchor::TopLeft,
            transform: Transform {
                translation: Vec3::new(-HALF_WIDTH, HALF_HEIGHT - 20.0, 0.0),
                ..default()
            },
            ..default()
        },InGameScreen
    ));

    // spawn the player's mana bar
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                "MP: ",
                TextStyle {
                    font: font_res.font.clone(),
                    font_size: 24.0,
                    color: Color::ALICE_BLUE,
                },
            ),
            text_anchor: Anchor::BottomLeft,
            transform: Transform {
                translation: Vec3::new(-HALF_WIDTH, -HALF_HEIGHT, 0.0),
                ..default()
            },
            ..default()
        },
        ManaBar,
        InGameScreen,
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
        ))).insert(InGameScreen);

    // create the left wall
    commands
        .spawn(RigidBody::Static)
        .insert(Collider::rectangle(1.0, WINDOW_HEIGHT))
        .insert(Name::new("Wall_Left"))
        .insert(TransformBundle::from(Transform::from_xyz(
            -HALF_WIDTH,
            0.0,
            0.0,
        ))).insert(InGameScreen);

    // create the right wall
    commands
        .spawn(RigidBody::Static)
        .insert(Collider::rectangle(1.0, WINDOW_HEIGHT))
        .insert(Name::new("Wall_Right"))
        .insert(TransformBundle::from(Transform::from_xyz(
            HALF_WIDTH, 0.0, 0.0,
        ))).insert(InGameScreen);

    // create the bottom
    commands
        .spawn(RigidBody::Static)
        .insert(Collider::rectangle(WINDOW_WIDTH, 1.0))
        .insert(Name::new("Wall_Bottom"))
        .insert(TransformBundle::from(Transform::from_xyz(
            0.0,
            -HALF_HEIGHT,
            0.0,
        ))).insert(InGameScreen);
}

fn spawn_player(mut commands: Commands, sprite_res: Res<SpriteAssets>) {
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
        .insert(CollisionLayers::new(
            GameLayer::Player,
            [GameLayer::Enemy, GameLayer::Gems],
        ))
        .insert(SpriteBundle {
            texture: sprite_res.player.clone(),
            ..default()
        })
        .insert(InputManagerBundle::with_map(
            PlayerAction::default_input_map(),
        ))
        .insert(Health {
            current: 10,
            max: 10,
        })
        .insert(Mana {
            current: 50,
            max: 50,
        })
        .insert(DamageDone(0))
        .insert(InGameScreen);
}

fn spawn_enemy(
    mut commands: Commands,
    sprite_res: Res<SpriteAssets>,
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
        .insert(CollisionLayers::new(
            GameLayer::Enemy,
            [GameLayer::Player, GameLayer::Minion],
        ))
        .insert(Position(ENEMY_POSITION))
        .insert(SpriteBundle {
            texture: sprite_res.enemy.clone(),
            ..default()
        })
        .insert(Health {
            current: 500,
            max: 500,
        })
        .insert(DamageDone(15))
        .with_children(|parent| {
            parent.spawn((
                Text2dBundle {
                    text: Text::from_section(
                        "HP: ",
                        TextStyle {
                            font: font_res.font.clone(),
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    ),
                    text_anchor: Anchor::BottomCenter,
                    transform: Transform {
                        translation: Vec3::new(0.0, ENEMY_RADIUS + 2.0, 0.0),
                        rotation: Quat::default(),
                        ..default()
                    },
                    ..default()
                },
                HealthBar,
            ));
        })
        .insert(InGameScreen);
}

fn minion_spawner(
    mut commands: Commands,
    mut er_spawn_minion: EventReader<SpawnMinionEvent>,
    player_pos_query: Query<&Transform, With<Player>>,
    sprite_res: Res<SpriteAssets>,
) {
    for event in er_spawn_minion.read() {
        let player_pos = player_pos_query.single().translation; // FIXME: this will panic if the player dies in the middle of spawning

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
            .insert(CollisionLayers::new(
                GameLayer::Minion,
                [GameLayer::Minion, GameLayer::Enemy],
            ))
            .insert(Position(minion_pos))
            .insert(SpriteBundle {
                texture: sprite_res.minion.clone(),
                ..default()
            })
            .insert(DamageDone(20))
            .insert(InGameScreen);
    }
}

fn handle_actions(
    mut commands: Commands,
    time: Res<Time>,
    action_query: Query<&ActionState<PlayerAction>, With<Player>>,
    mut player_xform_query: Query<&mut Position, With<Player>>,
    mut player_mana_query: Query<&mut Mana, With<Player>>,
    mut ew_spawn_minion: EventWriter<SpawnMinionEvent>,
    audio_assets: Res<AudioAssets>,
    // effects_channel: Res<AudioChannel<EffectsChannel>>
    audio: Res<Audio>,
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
                    && (position.x + move_delta.x > -HALF_WIDTH + PLAYER_RADIUS)
                {
                    position.x += move_delta.x;
                }

                // clamp y position within the window
                if (position.y + move_delta.y < HALF_HEIGHT - PLAYER_RADIUS)
                    && (position.y + move_delta.y > -HALF_HEIGHT + PLAYER_RADIUS)
                {
                    position.y += move_delta.y;
                }
            }
        }

        if action_state.just_pressed(&PlayerAction::SpawnMinions) {
            let mana_cost = 10;
            if let Ok(mut mana) = player_mana_query.get_single_mut() {
                // TODO: move this logic to the minion spawner
                if mana.current >= mana_cost {
                    // effects_channel.play(
                    //     audio_assets.spawn_minion.clone())
                    //     .with_volume(0.5);

                    let handle = audio
                        .play(audio_assets.spawn_minion.clone())
                        .with_volume(0.5)
                        .handle();
                    commands.insert_resource(AudioResource(handle));

                    mana.current -= mana_cost;

                    for i in 1..=2 {
                        ew_spawn_minion.send(SpawnMinionEvent(i as f32));
                    }
                } else {
                    // effects_channel.play(
                    //     audio_assets.oom.clone())
                    //     .with_volume(0.5);

                    let handle = audio
                        .play(audio_assets.oom.clone())
                        .with_volume(0.5)
                        .handle();
                    commands.insert_resource(AudioResource(handle));
                }
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
        // mana gem collisions
        if let Ok(mana_gem) = mana_gem_query.get(*entity2) {
            ew_mana_gained.send(ManaGainedEvent {
                player: *entity1,
                mana_gem: *entity2,
                amount: mana_gem.0,
            });
        }

        // damaging collisions
        if let Ok(damage) = damage_done_query.get(*entity1) {
            if damage.0 == 0 {
                trace!("Ignoring a zero damage event");
                continue;
            }

            // ignore minion-minion collisions
            if let Ok(_entity) = minion_query.get(*entity1) {
                // TODO: there has to be a better way of doing this
                if let Ok(_entity) = minion_query.get(*entity2) {
                    trace!("ignoring minion-minion collision");
                    continue;
                }
            }

            debug!(
                "Sending damage taken event from {:?} to {:?} for {} damage",
                entity1, entity2, damage.0
            );

            ew_damage_taken.send(DamageTakenEvent {
                giver: *entity1,
                receiver: *entity2,
                amount: damage.0,
            });
        }
    }
}

fn handle_damage_taken(
    mut commands: Commands,
    mut er_damage_taken: EventReader<DamageTakenEvent>,
    mut health_query: Query<(&mut Health, &Name), With<Health>>,
    player_query: Query<&Player>,
    enemy_query: Query<&Enemy>,
    minion_query: Query<(&Minion, &Name)>,
    audio_assets: Res<AudioAssets>,
    // effects_channel: Res<AudioChannel<EffectsChannel>>
    audio: Res<Audio>,
    mut next_state: ResMut<NextState<GameState>>,
    mut game_status: ResMut<GameStatus>,
) {
    for event in er_damage_taken.read() {
        if let Ok((mut health, name)) = health_query.get_mut(event.receiver) {
            // if the event giver is a minion, explode it before dealing the damage.
            if let Ok((_minion, name)) = minion_query.get(event.giver) {
                info!(
                    "{} ({:?}) explodes dealing {} damage.",
                    name, event.receiver, event.amount
                );

                commands.entity(event.giver).despawn();

                // effects_channel.play(
                //     audio_assets.minion_die.clone())
                //     .with_volume(0.5);

                let handle = audio
                    .play(audio_assets.minion_die.clone())
                    .with_volume(0.5)
                    .handle();
                commands.insert_resource(AudioResource(handle));
            }

            // subtract the damage done, but do not go below zero
            health.current = cmp::max(0, health.current - event.amount);

            info!(
                "{} ({:?}) takes {:?} damage from {:?} (final health = {:?})",
                name, event.receiver, event.amount, event.giver, health.current,
            );

            // if the health is equal to zero, the event receiver dies
            if health.current == 0 {
                info!("{} ({:?}) dies.", name, event.receiver);
                commands.entity(event.receiver).despawn_recursive();

                if let Ok(_player) = player_query.get(event.receiver) {
                    // effects_channel.play(
                    //     audio_assets.player_die.clone())
                    //     .with_volume(0.5);

                    let handle = audio
                        .play(audio_assets.player_die.clone())
                        .with_volume(0.5)
                        .handle();
                    commands.insert_resource(AudioResource(handle));

                    next_state.set(GameState::GameOver);
                    game_status.result = GameResult::Lose;

                } else if let Ok(_enemy) = enemy_query.get(event.receiver) {
                    // effects_channel.play(
                    //     audio_assets.enemy_die.clone())
                    //     .with_volume(0.5);

                    let handle = audio
                        .play(audio_assets.enemy_die.clone())
                        .with_volume(0.5)
                        .handle();
                    commands.insert_resource(AudioResource(handle));

                    next_state.set(GameState::GameOver);
                    game_status.result = GameResult::Win;
                }
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

fn update_mana_bar(mut mana_bar_query: Query<&mut Text, With<ManaBar>>, mana_query: Query<&Mana>) {
    for mana in mana_query.iter() {
        if let Ok(mut text) = mana_bar_query.get_single_mut() {
            text.sections[0].value = format!("MP: {:3}/{:3}", mana.current, mana.max);
        }
    }
}

fn setup_mana_spawning(mut commands: Commands) {
    commands.insert_resource(ManaSpawnConfig {
        timer: Timer::new(Duration::from_secs(2), TimerMode::Repeating),
    })
}

fn mana_spawner(
    mut commands: Commands,
    sprite_res: Res<SpriteAssets>,
    time: Res<Time>,
    mut config: ResMut<ManaSpawnConfig>,
    mana_gem_query: Query<&ManaGem>,
) {
    // tick the timer
    config.timer.tick(time.delta());

    // if the timer has elapsed, spawn a gem
    if config.timer.finished() && mana_gem_query.iter().len() <= 10 {
        let mut rng = rand::thread_rng();

        let gap = 5.0;
        let gem_x = rng.gen_range(-HALF_WIDTH + gap..=HALF_WIDTH - gap);
        let gem_y = rng.gen_range(-HALF_HEIGHT + gap..=HALF_HEIGHT - gap);
        let gem_pos = Vector::new(gem_x, gem_y);

        debug!("Spawning new mana gem at {}.", gem_pos);

        commands
            .spawn(ManaGem(10))
            .insert(Name::new("ManaGem"))
            .insert(RigidBody::Kinematic)
            .insert(Collider::circle(20.0))
            .insert(CollisionLayers::new(GameLayer::Gems, [GameLayer::Player]))
            .insert(Position(gem_pos))
            .insert(SpriteBundle {
                texture: sprite_res.mana_gem.clone(),
                ..default()
            }).insert(InGameScreen);
    }
}

fn handle_mana_gained(
    mut commands: Commands,
    mut er_mana_gained: EventReader<ManaGainedEvent>,
    mut mana_query: Query<(&mut Mana, &Name), With<Mana>>,
    audio_assets: Res<AudioAssets>,
    // effects_channel: Res<AudioChannel<EffectsChannel>>
    audio: Res<Audio>,
) {
    for event in er_mana_gained.read() {
        if let Ok((mut mana, name)) = mana_query.get_mut(event.player) {
            if mana.current < mana.max {
                // de-spawn the mana gem
                commands.entity(event.mana_gem).despawn();

                // effects_channel.play(
                //     audio_assets.mana_gem.clone())
                //     .with_volume(0.5);

                let handle = audio
                    .play(audio_assets.mana_gem.clone())
                    .with_volume(0.5)
                    .handle();
                commands.insert_resource(AudioResource(handle));

                // add the event amount, but do not go over the maximum
                mana.current = cmp::min(mana.max, mana.current + event.amount);

                info!(
                    "{} ({:?}) gains {:?} mana (final mana total = {:?})",
                    name, event.player, event.amount, mana.current,
                );
            }
        }
    }
}
