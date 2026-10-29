use crate::{errors::ParrotError, messaging::message::ParrotMessage, utils::create_response};
use serenity::{all::CommandInteraction, client::Context};

pub async fn restart(
    ctx: &Context,
    interaction: &mut CommandInteraction,
) -> Result<(), ParrotError> {
    create_response(&ctx.http, interaction, ParrotMessage::Restart).await?;
    log::info!("Restarting bot via /restart command");
    std::process::exit(0);
}
