use serenity::{model::gateway::GatewayIntents, prelude::TypeMapKey};
use songbird::serenity::SerenityInit;

use std::{collections::HashMap, env, error::Error};

use crate::{
    guild::{cache::GuildCacheMap, settings::GuildSettingsMap},
    handlers::SerenityHandler,
};

pub type HttpClient = reqwest::Client;

pub struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = HttpClient;
}

pub struct Client {
    client: serenity::Client,
}

impl Client {
    pub async fn default() -> Result<Client, Box<dyn Error>> {
        let token = env::var("DISCORD_TOKEN").expect("Fatality! DISCORD_TOKEN not set!");
        Client::new(token).await
    }

    pub async fn new(token: String) -> Result<Client, Box<dyn Error>> {
        let application_id = env::var("DISCORD_APP_ID")
            .expect("Fatality! DISCORD_APP_ID not set!")
            .parse()?;

        let gateway_intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

        let client = serenity::Client::builder(token, gateway_intents)
            .event_handler(SerenityHandler)
            .application_id(application_id)
            .register_songbird()
            .type_map_insert::<GuildCacheMap>(HashMap::default())
            .type_map_insert::<GuildSettingsMap>(HashMap::default())
            .type_map_insert::<HttpKey>(HttpClient::new())
            .await?;

        Ok(Client { client })
    }

    pub async fn start(&mut self) -> Result<(), serenity::Error> {
        self.client.start().await
    }
}
