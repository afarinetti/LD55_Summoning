use bevy::asset::Handle;
use bevy::prelude::{Image, Resource};
use bevy_asset_loader::asset_collection::AssetCollection;
use bevy_kira_audio::AudioSource;

#[derive(AssetCollection, Resource)]
pub struct AudioAssets {
    #[asset(path = "sounds/Action_-_Keep_Moving.ogg")]
    pub bgm: Handle<AudioSource>,

    #[asset(path = "sounds/SFX_-_magic_spell_01.ogg")]
    pub spawn_minion: Handle<AudioSource>,

    #[asset(path = "sounds/SFX_-_negative_03.ogg")]
    pub player_die: Handle<AudioSource>,

    #[asset(path = "sounds/SFX_-_positive_02.ogg")]
    pub enemy_die: Handle<AudioSource>,

    #[asset(path = "sounds/SFX_-_hit_basic_01.ogg")]
    pub minion_die: Handle<AudioSource>,

    #[asset(path = "sounds/SFX_-_coin_10.ogg")]
    pub mana_gem: Handle<AudioSource>,

    #[asset(path = "sounds/SFX_-_negative_04.ogg")]
    pub oom: Handle<AudioSource>,
}

#[derive(AssetCollection, Resource)]
pub struct SpriteAssets {
    #[asset(path = "images/Sprite-Player.png")]
    pub player: Handle<Image>,
    
    #[asset(path = "images/Sprite-Enemy.png")]
    pub enemy: Handle<Image>,
    
    #[asset(path = "images/Sprite-Bomb.png")]
    pub minion: Handle<Image>,
    
    #[asset(path = "images/Sprite-ManaGem.png")]
    pub mana_gem: Handle<Image>,
}
