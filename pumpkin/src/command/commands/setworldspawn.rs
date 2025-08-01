use crate::command::dispatcher::CommandError::InvalidConsumption;
use crate::command::{
    CommandExecutor, CommandSender,
    args::{
        Arg, ConsumedArgs, position_block::BlockPosArgumentConsumer,
        rotation::RotationArgumentConsumer,
    },
    dispatcher::CommandError,
    tree::{CommandTree, builder::argument},
};
use crate::server::Server;
use async_trait::async_trait;
use pumpkin_registry::VanillaDimensionType;
use pumpkin_util::{math::position::BlockPos, text::TextComponent};

const NAMES: [&str; 1] = ["setworldspawn"];

const DESCRIPTION: &str = "Sets the world spawn point.";

const ARG_BLOCK_POS: &str = "position";

const ARG_ANGLE: &str = "angle";

struct NoArgsWorldSpawnExecutor;

#[async_trait]
impl CommandExecutor for NoArgsWorldSpawnExecutor {
    async fn execute<'a>(
        &self,
        sender: &mut CommandSender,
        server: &crate::server::Server,
        _args: &ConsumedArgs<'a>,
    ) -> Result<(), CommandError> {
        let Some(block_pos) = sender.position() else {
            let level_info_guard = server.level_info.read().await;
            sender
                .send_message(TextComponent::translate(
                    "commands.setworldspawn.success",
                    [
                        TextComponent::text(level_info_guard.spawn_x.to_string()),
                        TextComponent::text(level_info_guard.spawn_y.to_string()),
                        TextComponent::text(level_info_guard.spawn_z.to_string()),
                        TextComponent::text(level_info_guard.spawn_angle.to_string()),
                    ],
                ))
                .await;

            return Ok(());
        };

        setworldspawn(sender, server, block_pos.to_block_pos(), 0.0).await
    }
}

struct DefaultWorldSpawnExecutor;

#[async_trait]
impl CommandExecutor for DefaultWorldSpawnExecutor {
    async fn execute<'a>(
        &self,
        sender: &mut CommandSender,
        server: &crate::server::Server,
        args: &ConsumedArgs<'a>,
    ) -> Result<(), CommandError> {
        let Some(Arg::BlockPos(block_pos)) = args.get(ARG_BLOCK_POS) else {
            return Err(InvalidConsumption(Some(ARG_BLOCK_POS.into())));
        };

        setworldspawn(sender, server, *block_pos, 0.0).await
    }
}

struct AngleWorldSpawnExecutor;

#[async_trait]
impl CommandExecutor for AngleWorldSpawnExecutor {
    async fn execute<'a>(
        &self,
        sender: &mut CommandSender,
        server: &Server,
        args: &ConsumedArgs<'a>,
    ) -> Result<(), CommandError> {
        let Some(Arg::BlockPos(block_pos)) = args.get(ARG_BLOCK_POS) else {
            return Err(InvalidConsumption(Some(ARG_BLOCK_POS.into())));
        };

        let Some(Arg::Rotation(_, yaw)) = args.get(ARG_ANGLE) else {
            return Err(InvalidConsumption(Some(ARG_ANGLE.into())));
        };

        setworldspawn(sender, server, *block_pos, *yaw).await
    }
}

async fn setworldspawn(
    sender: &mut CommandSender,
    server: &Server,
    block_pos: BlockPos,
    yaw: f32,
) -> Result<(), CommandError> {
    let Some(world) = sender.world().await else {
        return Err(CommandError::CommandFailed(Box::new(TextComponent::text(
            "Failed to get world.",
        ))));
    };

    match world.dimension_type {
        VanillaDimensionType::Overworld | VanillaDimensionType::OverworldCaves => {}
        _ => {
            sender
                .send_message(TextComponent::translate(
                    "commands.setworldspawn.failure.not_overworld",
                    [],
                ))
                .await;
            return Ok(());
        }
    }

    let mut level_info_guard = server.level_info.write().await;
    level_info_guard.spawn_x = block_pos.0.x;
    level_info_guard.spawn_y = block_pos.0.y;
    level_info_guard.spawn_z = block_pos.0.z;

    level_info_guard.spawn_angle = yaw;

    drop(level_info_guard);

    sender
        .send_message(TextComponent::translate(
            "commands.setworldspawn.success",
            [
                TextComponent::text(block_pos.0.x.to_string()),
                TextComponent::text(block_pos.0.y.to_string()),
                TextComponent::text(block_pos.0.z.to_string()),
                TextComponent::text(yaw.to_string()),
            ],
        ))
        .await;

    Ok(())
}

#[must_use]
pub fn init_command_tree() -> CommandTree {
    CommandTree::new(NAMES, DESCRIPTION)
        .execute(NoArgsWorldSpawnExecutor)
        .then(
            argument(ARG_BLOCK_POS, BlockPosArgumentConsumer)
                .execute(DefaultWorldSpawnExecutor)
                .then(
                    argument(ARG_ANGLE, RotationArgumentConsumer).execute(AngleWorldSpawnExecutor),
                ),
        )
}
