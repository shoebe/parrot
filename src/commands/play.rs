use crate::{
    commands::{skip::force_skip_top_track, summon::summon},
    errors::{verify, ParrotError},
    guild::settings::{GuildSettings, GuildSettingsMap},
    handlers::track_end::update_queue_messages,
    messaging::message::ParrotMessage,
    messaging::messages::{
        PLAY_QUEUE, PLAY_TOP, SPOTIFY_AUTH_FAILED, TRACK_DURATION, TRACK_TIME_TO_PLAY,
    },
    sources::spotify::{Spotify, SPOTIFY},
    utils::{
        compare_domains, create_now_playing_embed, create_response, edit_embed_response,
        edit_response, get_human_readable_timestamp,
    },
};
use serde_json::Value;
use serenity::{
    all::{CommandDataOptionValue, CommandInteraction, CreateEmbedFooter},
    builder::CreateEmbed,
    client::Context,
    prelude::Mutex,
};
use songbird::{
    input::{AuxMetadata, Input, YoutubeDl},
    tracks::{Track, TrackHandle},
    Call,
};
use std::{cmp::Ordering, error::Error as StdError, sync::Arc, time::Duration};
use tokio::process::Command;
use url::Url;

#[derive(Clone, Copy)]
pub enum Mode {
    End,
    Next,
    All,
    Reverse,
    Shuffle,
    Jump,
}

#[derive(Clone, Debug)]
pub enum QueryType {
    Keywords(String),
    KeywordList(Vec<String>),
    Link(String),
}

pub async fn play(ctx: &Context, interaction: &mut CommandInteraction) -> Result<(), ParrotError> {
    let args = interaction.data.options.clone();
    let first_arg = args.first().unwrap();

    let queue_mode = match first_arg.name.as_str() {
        "next" => Mode::Next,
        "all" => Mode::All,
        "reverse" => Mode::Reverse,
        "shuffle" => Mode::Shuffle,
        "jump" => Mode::Jump,
        _ => Mode::End,
    };

    dbg!(&first_arg);

    let url = match &first_arg.value {
        CommandDataOptionValue::String(s) => s.as_str(),
        CommandDataOptionValue::SubCommand(command_data_options) => {
            let sub = command_data_options.first().unwrap();
            sub.value.as_str().unwrap()
        }
        _ => {
            log::error!("could not parse value: {:?}", &first_arg.value);
            return Err(ParrotError::Other("error with command"));
        }
    };

    let guild_id = interaction.guild_id.unwrap();
    let manager = songbird::get(ctx).await.unwrap();

    // try to join a voice channel if not in one just yet
    summon(ctx, interaction, false).await?;
    let call = manager.get(guild_id).unwrap();

    // determine whether this is a link or a query string
    let query_type = match Url::parse(url) {
        Ok(url_data) => match url_data.host_str() {
            Some("open.spotify.com") => {
                let spotify = SPOTIFY.lock().await;
                let spotify = verify(spotify.as_ref(), ParrotError::Other(SPOTIFY_AUTH_FAILED))?;
                Some(Spotify::extract(spotify, url).await?)
            }
            Some(other) => {
                let mut data = ctx.data.write().await;
                let settings = data.get_mut::<GuildSettingsMap>().unwrap();
                let guild_settings = settings
                    .entry(guild_id)
                    .or_insert_with(|| GuildSettings::new(guild_id));

                let is_allowed = guild_settings
                    .allowed_domains
                    .iter()
                    .any(|d| compare_domains(d, other));

                let is_banned = guild_settings
                    .banned_domains
                    .iter()
                    .any(|d| compare_domains(d, other));

                if is_banned || (guild_settings.banned_domains.is_empty() && !is_allowed) {
                    return create_response(
                        &ctx.http,
                        interaction,
                        ParrotMessage::PlayDomainBanned {
                            domain: other.to_string(),
                        },
                    )
                    .await;
                }
                Some(QueryType::Link(url.to_string()))
            }
            None => None,
        },
        Err(_) => {
            let mut data = ctx.data.write().await;
            let settings = data.get_mut::<GuildSettingsMap>().unwrap();
            let guild_settings = settings
                .entry(guild_id)
                .or_insert_with(|| GuildSettings::new(guild_id));

            if guild_settings.banned_domains.contains("youtube.com")
                || (guild_settings.banned_domains.is_empty()
                    && !guild_settings.allowed_domains.contains("youtube.com"))
            {
                return create_response(
                    &ctx.http,
                    interaction,
                    ParrotMessage::PlayDomainBanned {
                        domain: "youtube.com".to_string(),
                    },
                )
                .await;
            }

            Some(QueryType::Keywords(url.to_string()))
        }
    };

    let query_type = verify(
        query_type,
        ParrotError::Other("Something went wrong while parsing your query!"),
    )?;

    // reply with a temporary message while we fetch the source
    // needed because interactions must be replied within 3s and queueing takes longer
    create_response(&ctx.http, interaction, ParrotMessage::Search).await?;

    let handler = call.lock().await;
    let queue_was_empty = handler.queue().is_empty();
    drop(handler);

    let http_client = {
        let data = ctx.data.read().await;
        data.get::<crate::client::HttpKey>().cloned().unwrap()
    };

    match queue_mode {
        Mode::End => match query_type.clone() {
            QueryType::Keywords(_) | QueryType::Link(_) => {
                let queue = enqueue_track(http_client, &call, &query_type).await?;
                update_queue_messages(&ctx.http, &ctx.data, &queue, guild_id).await;
            }
            QueryType::KeywordList(keywords_list) => {
                for keywords in keywords_list.iter() {
                    let queue = enqueue_track(
                        http_client.clone(),
                        &call,
                        &QueryType::Keywords(keywords.to_string()),
                    )
                    .await?;
                    update_queue_messages(&ctx.http, &ctx.data, &queue, guild_id).await;
                }
            }
        },
        Mode::Next => match query_type.clone() {
            QueryType::Keywords(_) | QueryType::Link(_) => {
                let queue = insert_track(http_client, &call, &query_type, 1).await?;
                update_queue_messages(&ctx.http, &ctx.data, &queue, guild_id).await;
            }
            QueryType::KeywordList(keywords_list) => {
                for (idx, keywords) in keywords_list.into_iter().enumerate() {
                    let queue = insert_track(
                        http_client.clone(),
                        &call,
                        &QueryType::Keywords(keywords),
                        idx + 1,
                    )
                    .await?;
                    update_queue_messages(&ctx.http, &ctx.data, &queue, guild_id).await;
                }
            }
        },
        Mode::Jump => match query_type.clone() {
            QueryType::Keywords(_) | QueryType::Link(_) => {
                let mut queue = enqueue_track(http_client, &call, &query_type).await?;

                if !queue_was_empty {
                    rotate_tracks(&call, 1).await.ok();
                    queue = force_skip_top_track(&call.lock().await).await?;
                }

                update_queue_messages(&ctx.http, &ctx.data, &queue, guild_id).await;
            }
            QueryType::KeywordList(keywords_list) => {
                let mut insert_idx = 1;

                for (i, keywords) in keywords_list.into_iter().enumerate() {
                    let mut queue = insert_track(
                        http_client.clone(),
                        &call,
                        &QueryType::Keywords(keywords),
                        insert_idx,
                    )
                    .await?;

                    if i == 0 && !queue_was_empty {
                        queue = force_skip_top_track(&call.lock().await).await?;
                    } else {
                        insert_idx += 1;
                    }

                    update_queue_messages(&ctx.http, &ctx.data, &queue, guild_id).await;
                }
            }
        },
        Mode::All | Mode::Reverse | Mode::Shuffle => match query_type.clone() {
            QueryType::Link(url) => {
                if let Ok(queue) = enqueue_track(http_client, &call, &QueryType::Link(url)).await {
                    update_queue_messages(&ctx.http, &ctx.data, &queue, guild_id).await;
                };
            }
            QueryType::KeywordList(keywords_list) => {
                for keywords in keywords_list.into_iter() {
                    let queue =
                        enqueue_track(http_client.clone(), &call, &QueryType::Keywords(keywords))
                            .await?;
                    update_queue_messages(&ctx.http, &ctx.data, &queue, guild_id).await;
                }
            }
            _ => {
                edit_response(&ctx.http, interaction, ParrotMessage::PlayAllFailed).await?;
                return Ok(());
            }
        },
    }

    let handler = call.lock().await;

    // refetch the queue after modification
    let queue = handler.queue().current_queue();
    drop(handler);

    match queue.len().cmp(&1) {
        Ordering::Greater => {
            let estimated_time = calculate_time_until_play(&queue, queue_mode).await.unwrap();

            match (query_type, queue_mode) {
                (QueryType::Link(_) | QueryType::Keywords(_), Mode::Next) => {
                    let track = queue.get(1).unwrap();
                    let embed = create_queued_embed(PLAY_TOP, track, estimated_time).await;

                    edit_embed_response(&ctx.http, interaction, embed).await?;
                }
                (QueryType::Link(_) | QueryType::Keywords(_), Mode::End) => {
                    let track = queue.last().unwrap();
                    let embed = create_queued_embed(PLAY_QUEUE, track, estimated_time).await;

                    edit_embed_response(&ctx.http, interaction, embed).await?;
                }
                (QueryType::KeywordList(_), _) => {
                    edit_response(&ctx.http, interaction, ParrotMessage::PlaylistQueued).await?;
                }
                (_, _) => {}
            }
        }
        Ordering::Equal => {
            let track = queue.first().unwrap();
            let embed = create_now_playing_embed(track).await;

            edit_embed_response(&ctx.http, interaction, embed).await?;
        }
        _ => unreachable!(),
    }

    Ok(())
}

async fn calculate_time_until_play(queue: &[TrackHandle], mode: Mode) -> Option<Duration> {
    if queue.is_empty() {
        return None;
    }

    let top_track = queue.first()?;
    let top_track_elapsed = top_track.get_info().await.unwrap().position;

    let top_track_duration = match top_track.data::<AuxMetadata>().duration {
        Some(duration) => duration,
        None => return Some(Duration::MAX),
    };

    match mode {
        Mode::Next => Some(top_track_duration - top_track_elapsed),
        _ => {
            let center = &queue[1..queue.len() - 1];
            let livestreams = center.len()
                - center
                    .iter()
                    .filter_map(|t| t.data::<AuxMetadata>().duration)
                    .count();

            // if any of the tracks before are livestreams, the new track will never play
            if livestreams > 0 {
                return Some(Duration::MAX);
            }

            let durations = center.iter().fold(Duration::ZERO, |acc, x| {
                acc + x.data::<AuxMetadata>().duration.unwrap()
            });

            Some(durations + top_track_duration - top_track_elapsed)
        }
    }
}

async fn create_queued_embed(
    title: &str,
    track: &TrackHandle,
    estimated_time: Duration,
) -> CreateEmbed {
    let metadata = track.data::<AuxMetadata>().clone();
    let footer_text = format!(
        "{}{}\n{}{}",
        TRACK_DURATION,
        get_human_readable_timestamp(metadata.duration),
        TRACK_TIME_TO_PLAY,
        get_human_readable_timestamp(Some(estimated_time))
    );

    CreateEmbed::default()
        .thumbnail(metadata.thumbnail.to_owned().unwrap())
        .field(
            title,
            format!(
                "[**{}**]({})",
                metadata.title.to_owned().unwrap(),
                metadata.source_url.to_owned().unwrap()
            ),
            false,
        )
        .footer(CreateEmbedFooter::new(footer_text))
}

async fn get_track_source(
    http_client: crate::client::HttpClient,
    query_type: QueryType,
) -> Result<Box<dyn Iterator<Item = YoutubeDl<'static>> + Send + Sync>, ParrotError> {
    dbg!(&query_type);

    match query_type {
        QueryType::Link(query) => {
            let url = Url::parse(&query).unwrap();
            let list = url.query_pairs().find(|(name, _val)| name == "list");
            if let Some((_name, list)) = list {
                let playlist_query = format!("https://www.youtube.com/playlist?list={list}");

                let args = vec![playlist_query.as_str(), "--flat-playlist", "-j"];

                let output = Command::new("yt-dlp")
                    .args(args)
                    .output()
                    .await
                    .map_err(|e| {
                        log::error!("err: {e:?}");
                        ParrotError::Other("error querying yt-dlp for playlist")
                    })?;

                if !output.status.success() {
                    log::error!(
                        "{} failed with non-zero status code: {}",
                        "yt-dlp",
                        std::str::from_utf8(&output.stderr[..]).unwrap_or("<no error message>")
                    );
                    return Err(ParrotError::Other("failed"));
                }

                let s = String::from_utf8_lossy(&output.stdout);

                let urls = s
                    .lines()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|line| {
                        let entry: Value = serde_json::from_str(&line).unwrap();
                        entry
                            .get("webpage_url")
                            .unwrap()
                            .as_str()
                            .unwrap()
                            .to_string()
                    })
                    .map(move |url| {
                        log::info!("url: {url}");
                        YoutubeDl::new(http_client.clone(), url)
                    });

                Ok(Box::new(urls))
            } else {
                let yt = YoutubeDl::new(http_client, query);
                Ok(Box::new(std::iter::once(yt)))
            }
        }
        QueryType::Keywords(query) => {
            let yt = YoutubeDl::new_search(http_client, query);
            Ok(Box::new(std::iter::once(yt)))
        }

        _ => unreachable!(),
    }
}

async fn enqueue_track(
    http_client: crate::client::HttpClient,
    call: &Arc<Mutex<Call>>,
    query_type: &QueryType,
) -> Result<Vec<TrackHandle>, ParrotError> {
    // safeguard against ytdl dying on a private/deleted video and killing the playlist
    let source = get_track_source(http_client, query_type.clone()).await?;

    for s in source {
        let mut s = Input::from(s);
        let metadata = s.aux_metadata().await.map_err(ParrotError::Metadata)?;

        let track = Track::new_with_data(s, Arc::new(metadata));

        let mut handler = call.lock().await;
        handler.enqueue(track).await;
    }

    let handler = call.lock().await;
    dbg!(handler.queue().current_queue().len());

    Ok(handler.queue().current_queue())
}

async fn insert_track(
    http_client: crate::client::HttpClient,
    call: &Arc<Mutex<Call>>,
    query_type: &QueryType,
    idx: usize,
) -> Result<Vec<TrackHandle>, ParrotError> {
    let handler = call.lock().await;
    let queue_size = handler.queue().len();
    drop(handler);

    if queue_size <= 1 {
        let queue = enqueue_track(http_client, call, query_type).await?;
        return Ok(queue);
    }

    verify(
        idx > 0 && idx <= queue_size,
        ParrotError::NotInRange("index", idx as isize, 1, queue_size as isize),
    )?;

    enqueue_track(http_client, call, query_type).await?;

    let handler = call.lock().await;
    handler.queue().modify_queue(|queue| {
        let back = queue.pop_back().unwrap();
        queue.insert(idx, back);
    });

    Ok(handler.queue().current_queue())
}

async fn rotate_tracks(
    call: &Arc<Mutex<Call>>,
    n: usize,
) -> Result<Vec<TrackHandle>, Box<dyn StdError>> {
    let handler = call.lock().await;

    verify(
        handler.queue().len() > 2,
        ParrotError::Other("cannot rotate queues smaller than 3 tracks"),
    )?;

    handler.queue().modify_queue(|queue| {
        let mut not_playing = queue.split_off(1);
        not_playing.rotate_right(n);
        queue.append(&mut not_playing);
    });

    Ok(handler.queue().current_queue())
}
