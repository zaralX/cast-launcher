use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;

use discord_rich_presence::activity::{Activity, ActivityType, Assets, Button, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use serde_json::json;
use tokio::sync::Mutex;

use cast_core::presence::{self, Playing, Presence, PresenceText};

use crate::state::AppState;

const APP_ID: &str = "1559513341964062812";

const RETRY: Duration = Duration::from_secs(15);

const CHECK: Duration = Duration::from_secs(60);

pub struct Discord {
    sender: Sender<Option<Presence>>,
    text: Mutex<Option<PresenceText>>,
}

impl Discord {
    pub fn start() -> Self {
        let (sender, receiver) = mpsc::channel();

        let spawned = std::thread::Builder::new()
            .name("discord-presence".into())
            .spawn(move || Worker::default().run(receiver));

        if let Err(error) = spawned {
            log::warn!("Failed to start the Discord status thread: {error}");
        }

        Self {
            sender,
            text: Mutex::new(None),
        }
    }

    pub async fn set_text(&self, text: PresenceText) {
        *self.text.lock().await = Some(text);
    }
}

pub async fn refresh(state: &Arc<AppState>) {
    let text = state.discord.text.lock().await;

    let Some(text) = text.as_ref() else {
        return;
    };

    let config = state.config().await.discord;
    let running = state.processes.running().await;

    let mut game = None;

    if let Some(running) = presence::latest(&running).filter(|_| config.enabled) {
        if let Ok(instance) = state.instances.get(&running.instance_id).await {
            let icon = instance.icon.trim();
            let icon_url = presence::icon_url(icon, state.icon_sources.link(icon).await);

            game = Some(Playing::new(&instance, running.started_at, icon_url));
        }
    }

    let wanted = presence::build(config, text, game.as_ref());

    if state.discord.sender.send(wanted).is_err() {
        log::warn!("The Discord status thread is not running");
    }
}

#[derive(Default)]
struct Worker {
    wanted: Option<Presence>,
    client: Option<DiscordIpcClient>,
    online: bool,
    missing: bool,
}

impl Worker {
    fn run(mut self, receiver: Receiver<Option<Presence>>) {
        loop {
            let wait = if self.client.is_some() { CHECK } else { RETRY };

            match receiver.recv_timeout(wait) {
                Ok(next) => {
                    let next = receiver.try_iter().last().unwrap_or(next);

                    if next == self.wanted && (next.is_none() || self.online) {
                        continue;
                    }

                    self.wanted = next;
                    self.apply(true);
                }
                Err(RecvTimeoutError::Timeout) => {
                    if self.wanted.is_some() {
                        self.apply(false);
                    }
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }

        self.disconnect();
    }

    fn apply(&mut self, changed: bool) {
        let Some(presence) = self.wanted.clone() else {
            if let Some(client) = self.client.as_mut() {
                if let Err(error) = client.clear_activity().and_then(|_| client.recv()) {
                    log::warn!("Failed to clear the Discord status: {error}");
                }
            }

            self.disconnect();
            return;
        };

        let Some(client) = self.connect() else {
            return;
        };

        match client
            .set_activity(activity(&presence))
            .and_then(|_| client.recv())
        {
            Ok((_, reply)) if reply["evt"] == "ERROR" => {
                if changed {
                    log::warn!("Discord refused the status: {}", reply["data"]["message"]);
                }
            }
            Ok((_, reply)) => {
                if !self.online {
                    log::info!("Discord accepted the status: {}", reply["data"]);
                    self.online = true;
                }
            }
            Err(error) => {
                if self.online || changed {
                    log::info!("Lost the connection to Discord: {error}");
                }

                self.online = false;
                self.client = None;
            }
        }
    }

    fn connect(&mut self) -> Option<&mut DiscordIpcClient> {
        if self.client.is_none() {
            let mut client = DiscordIpcClient::new(APP_ID);

            let ready = client
                .connect_ipc()
                .and_then(|_| client.send(json!({ "v": 1, "client_id": APP_ID }), 0))
                .and_then(|_| client.recv());

            let failure = match ready {
                Ok((_, reply)) if reply["evt"] == "READY" => {
                    log::info!(
                        "Connected to Discord at {}",
                        reply["data"]["config"]["api_endpoint"]
                    );
                    self.missing = false;
                    self.client = Some(client);
                    return self.client.as_mut();
                }
                Ok((_, reply)) => reply["message"].to_string(),
                Err(error) => error.to_string(),
            };

            if !self.missing {
                log::info!("Discord is not available: {failure}");
                self.missing = true;
            }
        }

        self.client.as_mut()
    }

    fn disconnect(&mut self) {
        self.online = false;

        if let Some(mut client) = self.client.take() {
            if let Err(error) = client.close() {
                log::warn!("Failed to close the Discord connection: {error}");
            }
        }
    }
}

fn activity(presence: &Presence) -> Activity<'_> {
    let mut assets = Assets::new()
        .large_image(presence.large_image.as_str())
        .large_text(presence.large_text.as_str());

    if let (Some(image), Some(text)) = (&presence.small_image, &presence.small_text) {
        assets = assets.small_image(image.as_str()).small_text(text.as_str());
    }

    let mut activity = Activity::new()
        .activity_type(ActivityType::Playing)
        .assets(assets);

    if let Some(details) = &presence.details {
        activity = activity.details(details.as_str());
    }

    if let Some(state) = &presence.state {
        activity = activity.state(state.as_str());
    }

    if let Some(started_at) = presence.started_at {
        activity = activity.timestamps(Timestamps::new().start(started_at as i64));
    }

    if let Some(label) = &presence.button {
        activity = activity.buttons(vec![Button::new(label.as_str(), presence::SITE)]);
    }

    activity
}
