use std::fmt::Display;

use serenity::model::mention::Mention;

use crate::messaging::messages::*;

const RELEASES_LINK: &str = "https://github.com/aquelemiguel/parrot/releases";

#[derive(Debug)]
pub enum ParrotMessage {
    AutopauseOff,
    AutopauseOn,
    Clear,
    Error,
    Leaving,
    LoopDisable,
    LoopEnable,
    NowPlaying,
    Pause,
    PlayAllFailed,
    PlayDomainBanned { domain: String },
    PlaylistQueued,
    String(String),
    RemoveMultiple,
    Resume,
    Search,
    Seek { timestamp: String },
    Shuffle,
    Skip,
    SkipAll,
    SkipTo { title: String, url: String },
    Stop,
    Summon { mention: Mention },
    Version { current: String },
    VoteSkip { mention: Mention, missing: usize },
}

impl Display for ParrotMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParrotMessage::AutopauseOff => {
                        f.write_str(AUTOPAUSE_OFF)
                    },
            ParrotMessage::AutopauseOn => f.write_str(AUTOPAUSE_ON),
            ParrotMessage::Clear => f.write_str(CLEARED),
            ParrotMessage::Error => f.write_str(ERROR),
            ParrotMessage::Leaving => f.write_str(LEAVING),
            ParrotMessage::LoopDisable => f.write_str(LOOP_DISABLED),
            ParrotMessage::LoopEnable => f.write_str(LOOP_ENABLED),
            ParrotMessage::NowPlaying => f.write_str(QUEUE_NOW_PLAYING),
            ParrotMessage::Pause => f.write_str(PAUSED),
            ParrotMessage::PlaylistQueued => f.write_str(PLAY_PLAYLIST),
            ParrotMessage::PlayAllFailed => f.write_str(PLAY_ALL_FAILED),
            ParrotMessage::PlayDomainBanned { domain } => {
                        f.write_str(&format!("⚠️ **{domain}** {PLAY_FAILED_BLOCKED_DOMAIN}"))
                    }
            ParrotMessage::Search => f.write_str(SEARCHING),
            ParrotMessage::RemoveMultiple => f.write_str(REMOVED_QUEUE_MULTIPLE),
            ParrotMessage::Resume => f.write_str(RESUMED),
            ParrotMessage::Shuffle => f.write_str(SHUFFLED_SUCCESS),
            ParrotMessage::Stop => f.write_str(STOPPED),
            ParrotMessage::VoteSkip { mention, missing } => f.write_str(&format!(
                        "{SKIP_VOTE_EMOJI}{mention} {SKIP_VOTE_USER} {missing} {SKIP_VOTE_MISSING}"
                    )),
            ParrotMessage::Seek { timestamp } => f.write_str(&format!("{SEEKED} **{timestamp}**!")),
            ParrotMessage::Skip => f.write_str(SKIPPED),
            ParrotMessage::SkipAll => f.write_str(SKIPPED_ALL),
            ParrotMessage::SkipTo { title, url } => {
                        f.write_str(&format!("{SKIPPED_TO} [**{title}**]({url})!"))
                    }
            ParrotMessage::Summon { mention } => f.write_str(&format!("{JOINING} **{mention}**!")),
            ParrotMessage::Version { current } => f.write_str(&format!(
                        "{VERSION} [{current}]({RELEASES_LINK}/tag/v{current})\n{VERSION_LATEST}({RELEASES_LINK}/latest)"
                    )),
            ParrotMessage::String(string) => {
                write!(f, "{string}")
            },
        }
    }
}
