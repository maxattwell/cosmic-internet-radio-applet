// SPDX-License-Identifier: MPL-2.0

use crate::config::Config;
use crate::player::Player;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::{window::Id, Limits, Subscription};
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::prelude::*;
use cosmic::widget;
use futures_util::{SinkExt, StreamExt};
use gstreamer::{MessageView, State};
use gstreamer::prelude::*;

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// The popup id.
    popup: Option<Id>,
    /// Configuration data that persists between application runs.
    config: Config,
    /// The audio player.
    player: Option<Player>,
    /// Current playback state.
    play_state: State,
}

impl Default for AppModel {
    fn default() -> Self {
        let player = match Player::new() {
            Ok(p) => Some(p),
            Err(e) => {
                tracing::error!("Failed to initialize player: {}", e);
                None
            }
        };

        Self {
            core: Default::default(),
            popup: Default::default(),
            config: Default::default(),
            player,
            play_state: State::Null,
        }
    }
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    SubscriptionChannel,
    UpdateConfig(Config),
    TogglePlayback,
    PlayerStateChanged(State),
    MetadataUpdated(gstreamer::TagList),
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "com.github.maxattwell.cosmic-internet-radio-applet";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        // Construct the app model with the runtime's core.
        let app = AppModel {
            core,
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((_errors, config)) => {
                        config
                    }
                })
                .unwrap_or_default(),
            ..Default::default()
        };

        (app, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// The applet's button in the panel will be drawn using the main view method.
    /// This view should emit messages to toggle the applet's popup window, which will
    /// be drawn using the `view_window` method.
    fn view(&self) -> Element<'_, Self::Message> {
        if self.player.is_none() {
            return self.core
                .applet
                .icon_button("dialog-error-symbolic")
                .into();
        }

        let icon = "audio-card-symbolic";

        self.core
            .applet
            .icon_button(icon)
            .on_press(Message::TogglePopup)
            .into()
    }

    /// The applet's popup window will be drawn using this view method. If there are
    /// multiple poups, you may match the id parameter to determine which popup to
    /// create a view for.
    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        if self.player.is_none() {
            return self.core.applet.popup_container(
                widget::text::text("Audio initialization failed. Check logs.")
            ).into();
        }

        let (play_pause_icon, play_pause_label) = match self.play_state {
            State::Playing | State::Paused => {
                ("media-playback-stop-symbolic", "Stop")
            }
            _ => ("media-playback-start-symbolic", "Play"),
        };

        let content_list = widget::list_column()
            .padding(5)
            .spacing(0)
            .add(widget::settings::item(
                play_pause_label,
                widget::button::icon(widget::icon::from_name(play_pause_icon))
                    .on_press(Message::TogglePlayback),
            ));

        self.core.applet.popup_container(content_list).into()
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-lived async tasks running in the background which
    /// emit messages to the application through a channel. They may be conditionally
    /// activated by selectively appending to the subscription batch, and will
    /// continue to execute for the duration that they remain in the batch.
    fn subscription(&self) -> Subscription<Self::Message> {
        struct MySubscription;
        struct PlayerSubscription;

        let mut subs = vec![
            // Create a subscription which emits updates through a channel.
            Subscription::run_with_id(
                std::any::TypeId::of::<MySubscription>(),
                cosmic::iced::stream::channel(4, move |mut channel| async move {
                    _ = channel.send(Message::SubscriptionChannel).await;

                    futures_util::future::pending().await
                }),
            ),
            // Watch for application configuration changes.
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| {
                    Message::UpdateConfig(update.config)
                }),
        ];

        if let Some(player) = &self.player {
            let bus = player.bus();
            let pipeline = player.pipeline().clone();

            subs.push(Subscription::run_with_id(
                std::any::TypeId::of::<PlayerSubscription>(),
                cosmic::iced::stream::channel(10, move |mut channel| async move {
                    let mut bus_stream = bus.stream();

                    while let Some(msg) = bus_stream.next().await {
                        match msg.view() {
                            MessageView::StateChanged(state_changed) => {
                                if let Some(src) = msg.src() {
                                    if let Some(src_pipeline) = src.downcast_ref::<gstreamer::Pipeline>() {
                                        if src_pipeline == &pipeline {
                                            let new_state = state_changed.current();
                                            let _ = channel.send(Message::PlayerStateChanged(new_state)).await;
                                        }
                                    }
                                }
                            }
                            MessageView::Tag(tags_msg) => {
                                let tags = tags_msg.tags();
                                let _ = channel.send(Message::MetadataUpdated(tags)).await;
                            }
                            MessageView::Error(err) => {
                                tracing::error!("GStreamer error: {} ({:?})", err.error(), err.debug());
                                let _ = channel.send(Message::PlayerStateChanged(State::Null)).await;
                            }
                            _ => (),
                        }
                    }

                    futures_util::future::pending().await
                }),
            ));
        }

        Subscription::batch(subs)
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime. The application will not exit until all
    /// tasks are finished.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::SubscriptionChannel => {
                // For example purposes only.
            }
            Message::UpdateConfig(config) => {
                self.config = config;
            }
            Message::TogglePlayback => {
                if let Some(player) = &self.player {
                    match self.play_state {
                        State::Playing | State::Paused => {
                            if let Err(e) = player.stop() {
                                tracing::error!("Failed to stop playback: {}", e);
                            }
                        }
                        _ => {
                            if let Err(e) = player.play("http://icecast.radiofrance.fr/fip-midfi.mp3") {
                                tracing::error!("Failed to start playback: {}", e);
                            }
                        }
                    }
                }
            },
            Message::PlayerStateChanged(state) => {
                self.play_state = state;
            }
            Message::MetadataUpdated(_tags) => {
                // Placeholder for metadata extraction
            }
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(372.0)
                        .min_width(300.0)
                        .min_height(200.0)
                        .max_height(1080.0);
                    get_popup(popup_settings)
                }
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }
}
