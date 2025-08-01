use std::{
    num::{NonZeroU8, NonZeroU32},
    sync::Arc,
};

use pumpkin_config::BASIC_CONFIG;
use pumpkin_protocol::{
    bedrock::client::network_chunk_publisher_update::CNetworkChunkPublisherUpdate,
    java::client::play::{CCenterChunk, CUnloadChunk},
};
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::cylindrical_chunk_iterator::Cylindrical;

use crate::{entity::player::Player, net::ClientPlatform};

pub async fn get_view_distance(player: &Player) -> NonZeroU8 {
    player
        .config
        .read()
        .await
        .view_distance
        .clamp(NonZeroU8::new(2).unwrap(), BASIC_CONFIG.view_distance)
}

pub async fn update_position(player: &Arc<Player>) {
    let entity = &player.living_entity.entity;
    let pos = entity.pos.load();

    let view_distance = get_view_distance(player).await;
    let new_chunk_center = entity.chunk_pos.load();

    let old_cylindrical = player.watched_section.load();
    let new_cylindrical = Cylindrical::new(new_chunk_center, view_distance);

    if old_cylindrical != new_cylindrical {
        match &player.client {
            ClientPlatform::Java(client) => {
                client
                    .send_packet_now(&CCenterChunk {
                        chunk_x: new_chunk_center.x.into(),
                        chunk_z: new_chunk_center.y.into(),
                    })
                    .await;
            }
            ClientPlatform::Bedrock(client) => {
                client
                    .send_game_packet(&CNetworkChunkPublisherUpdate::new(
                        BlockPos::new(pos.x as i32, pos.y as i32, pos.z as i32),
                        NonZeroU32::from(view_distance).get(),
                    ))
                    .await;
            }
        }
        let mut loading_chunks = Vec::new();
        let mut unloading_chunks = Vec::new();
        Cylindrical::for_each_changed_chunk(
            old_cylindrical,
            new_cylindrical,
            |chunk_pos| {
                loading_chunks.push(chunk_pos);
            },
            |chunk_pos| {
                unloading_chunks.push(chunk_pos);
            },
        );

        // Make sure the watched section and the chunk watcher updates are async atomic. We want to
        // ensure what we unload when the player disconnects is correct.
        let level = &entity.world.read().await.level;
        level.mark_chunks_as_newly_watched(&loading_chunks).await;
        let chunks_to_clean = level.mark_chunks_as_not_watched(&unloading_chunks).await;

        {
            // After marking the chunks as watched, remove chunks that we are already in the process
            // of sending.
            let chunk_manager = player.chunk_manager.lock().await;
            loading_chunks.retain(|pos| !chunk_manager.is_chunk_pending(pos));
        };

        player.watched_section.store(new_cylindrical);

        if !chunks_to_clean.is_empty() {
            level.clean_chunks(&chunks_to_clean).await;
            for chunk in unloading_chunks {
                player
                    .client
                    .enqueue_packet(&CUnloadChunk::new(chunk.x, chunk.y))
                    .await;
            }
        }

        if !loading_chunks.is_empty() {
            entity.world.read().await.spawn_world_chunks(
                player.clone(),
                loading_chunks,
                new_chunk_center,
            );
        }
    }
}
