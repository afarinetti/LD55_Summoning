use bevy::asset::{AssetServer, Handle};
use bevy::log::info;
use bevy::prelude::{Commands, Res, Resource};
use bevy_kira_audio::prelude::*;
use crate::loading::AudioAssets;

#[derive(Resource)]
pub struct AudioResource(pub Handle<AudioInstance>);

// #[derive(Resource)]
// pub struct MusicChannel;

// #[derive(Resource)]
// pub struct EffectsChannel;

pub fn play_bgm(
    mut commands: Commands,
    audio_assets: Res<AudioAssets>,
    // music_channel: Res<AudioChannel<MusicChannel>>,
    audio: Res<Audio>,
) {
    info!("Attempting to play audio!");
    let handle = audio
        .play(audio_assets.bgm.clone())
        .looped()
        .with_volume(0.3)
        .handle();
    info!("Spawned auto resource: {:?}", handle.clone());
    commands.insert_resource(AudioResource(handle));
    // music_channel.pause();
    // music_channel
    //     .play(audio_assets.bgm.clone())
    //     .looped()
    //     .with_volume(0.25);
}
