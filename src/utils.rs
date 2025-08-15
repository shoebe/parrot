use serenity::{
    all::{
        CommandInteraction, CreateEmbedAuthor, CreateEmbedFooter, CreateInteractionResponse,
        CreateInteractionResponseMessage, EditInteractionResponse, Message,
    },
    builder::CreateEmbed,
    http::{Http, HttpError},
    Error,
};
use songbird::{input::AuxMetadata, tracks::TrackHandle};
use std::{sync::Arc, time::Duration};
use url::Url;

use crate::{errors::ParrotError, messaging::message::ParrotMessage};

pub async fn create_response(
    http: &Arc<Http>,
    interaction: &mut CommandInteraction,
    message: ParrotMessage,
) -> Result<(), ParrotError> {
    let embed = CreateEmbed::default().description(format!("{message}"));
    create_embed_response(http, interaction, embed).await
}

pub async fn create_response_text(
    http: &Arc<Http>,
    interaction: &mut CommandInteraction,
    content: &str,
) -> Result<(), ParrotError> {
    let embed = CreateEmbed::default().description(content);
    create_embed_response(http, interaction, embed).await
}

pub async fn edit_response(
    http: &Arc<Http>,
    interaction: &mut CommandInteraction,
    message: ParrotMessage,
) -> Result<Message, ParrotError> {
    let embed = CreateEmbed::default().description(format!("{message}"));
    edit_embed_response(http, interaction, embed).await
}

pub async fn edit_response_text(
    http: &Arc<Http>,
    interaction: &mut CommandInteraction,
    content: &str,
) -> Result<Message, ParrotError> {
    let embed = CreateEmbed::default().description(content);
    edit_embed_response(http, interaction, embed).await
}

pub async fn create_embed_response(
    http: &Arc<Http>,
    interaction: &mut CommandInteraction,
    embed: CreateEmbed,
) -> Result<(), ParrotError> {
    match interaction
        .create_response(
            &http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().add_embed(embed.clone()),
            ),
        )
        .await
    {
        Ok(val) => Ok(val),
        Err(err) => match err {
            serenity::Error::Http(ref e) => match &*e {
                HttpError::UnsuccessfulRequest(req) => match req.error.code {
                    40060 => edit_embed_response(http, interaction, embed)
                        .await
                        .map(|_| ()),
                    _ => Err(ParrotError::Serenity(err)),
                },
                _ => Err(ParrotError::Serenity(err)),
            },
            _ => Err(ParrotError::Serenity(err)),
        },
    }
}

pub async fn edit_embed_response(
    http: &Arc<Http>,
    interaction: &mut CommandInteraction,
    embed: CreateEmbed,
) -> Result<Message, ParrotError> {
    interaction
        .edit_response(
            &http,
            EditInteractionResponse::new().content(" ").add_embed(embed),
        )
        .await
        .map_err(Into::into)
}

pub async fn create_now_playing_embed(track: &TrackHandle) -> CreateEmbed {
    let metadata = track.data::<AuxMetadata>();

    let embed = CreateEmbed::default()
        .title(metadata.title.to_owned().unwrap_or_default())
        .author(CreateEmbedAuthor::new(
            ParrotMessage::NowPlaying.to_string(),
        ))
        .url(metadata.source_url.as_ref().unwrap());

    let position = get_human_readable_timestamp(Some(track.get_info().await.unwrap().position));
    let duration = get_human_readable_timestamp(metadata.duration);

    let embed = embed.field("Progress", format!(">>> {} / {}", position, duration), true);

    let embed = match &metadata.channel {
        Some(channel) => embed.field("Channel", format!(">>> {}", channel), true),
        None => embed.field("Channel", ">>> N/A", true),
    };

    let embed = embed.thumbnail(metadata.thumbnail.to_owned().unwrap());

    let source_url = metadata.source_url.as_ref().unwrap();

    let (footer_text, footer_icon_url) = get_footer_info(source_url);
    let embed = embed.footer(CreateEmbedFooter::new(footer_text).icon_url(footer_icon_url));

    embed
}

pub fn get_footer_info(url: &str) -> (String, String) {
    let url_data = Url::parse(url).unwrap();
    let domain = url_data.host_str().unwrap();

    // remove www prefix because it looks ugly
    let domain = domain.replace("www.", "");

    (
        format!("Streaming via {}", domain),
        format!("https://www.google.com/s2/favicons?domain={}", domain),
    )
}

pub fn get_human_readable_timestamp(duration: Option<Duration>) -> String {
    match duration {
        Some(duration) if duration == Duration::MAX => "∞".to_string(),
        Some(duration) => {
            let seconds = duration.as_secs() % 60;
            let minutes = (duration.as_secs() / 60) % 60;
            let hours = duration.as_secs() / 3600;

            if hours < 1 {
                format!("{:02}:{:02}", minutes, seconds)
            } else {
                format!("{}:{:02}:{:02}", hours, minutes, seconds)
            }
        }
        None => "∞".to_string(),
    }
}

pub fn compare_domains(domain: &str, subdomain: &str) -> bool {
    subdomain == domain || subdomain.ends_with(domain)
}
