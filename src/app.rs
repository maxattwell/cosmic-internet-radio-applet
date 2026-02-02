// SPDX-License-Identifier: MPL-2.0

use crate::channels::{self, Channel, ChannelList};
use crate::config::Config;
use crate::player::Player;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::{window::Id, Limits, Subscription, Task};
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
    /// List of radio channels.
    channels: Vec<Channel>,
    /// Index of the currently playing channel (None if stopped).
    current_channel_idx: Option<usize>,
    /// Error message to display (if any).
    error_message: Option<String>,
    /// Whether we're currently in "add station" mode.
    adding_station: bool,
    /// New station name input.
    new_station_name: String,
    /// New station URL input.
    new_station_url: String,
    /// Validation error for new station form.
    new_station_error: Option<String>,
    /// Index of station being edited (None if not editing).
    editing_station_idx: Option<usize>,
    /// Edit form station name input.
    edit_station_name: String,
    /// Edit form station URL input.
    edit_station_url: String,
    /// Validation error for edit form.
    edit_station_error: Option<String>,
    /// Index of station pending deletion (for confirmation).
    deleting_station_idx: Option<usize>,
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
            channels: Vec::new(),
            current_channel_idx: None,
            error_message: None,
            adding_station: false,
            new_station_name: String::new(),
            new_station_url: String::new(),
            new_station_error: None,
            editing_station_idx: None,
            edit_station_name: String::new(),
            edit_station_url: String::new(),
            edit_station_error: None,
            deleting_station_idx: None,
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
    /// Play a specific channel by its index in the channels list
    PlayChannel(usize),
    /// Stop playback and clear current channel
    StopPlayback,
    /// Channels loaded from file
    ChannelsLoaded(Vec<Channel>),
    /// Error loading channels
    ChannelError(String),
    /// Toggle add station form visibility
    ToggleAddStation,
    /// New station name changed
    NewStationNameChanged(String),
    /// New station URL changed
    NewStationUrlChanged(String),
    /// Save the new station
    SaveNewStation,
    /// Cancel adding station
    CancelAddStation,
    /// Start editing a station
    StartEditStation(usize),
    /// Edit form station name changed
    EditStationNameChanged(String),
    /// Edit form station URL changed
    EditStationUrlChanged(String),
    /// Save edited station
    SaveEditStation,
    /// Cancel editing station
    CancelEditStation,
    /// Start deleting a station (show confirmation)
    StartDeleteStation(usize),
    /// Confirm and delete station
    ConfirmDeleteStation,
    /// Cancel deletion
    CancelDeleteStation,
}

/// Helper methods for AppModel
impl AppModel {
    /// View for the add station form
    fn view_add_station_form(&self) -> Element<'_, Message> {
        let mut form = widget::column()
            .padding(10)
            .spacing(10);

        // Header
        form = form.push(
            widget::text::text("Add New Station")
                .size(16)
        );

        // Name input
        form = form.push(
            widget::column()
                .spacing(5)
                .push(widget::text::text("Station Name:").size(12))
                .push(
                    widget::text_input("e.g., My Radio Station", &self.new_station_name)
                        .on_input(Message::NewStationNameChanged)
                )
        );

        // URL input
        form = form.push(
            widget::column()
                .spacing(5)
                .push(widget::text::text("Stream URL:").size(12))
                .push(
                    widget::text_input("e.g., http://example.com/stream.mp3", &self.new_station_url)
                        .on_input(Message::NewStationUrlChanged)
                )
        );

        // Error message
        if let Some(error) = &self.new_station_error {
            form = form.push(
                widget::text::text(format!("Error: {}", error))
                    .size(12)
            );
        }

        // Buttons
        form = form.push(
            widget::row()
                .spacing(10)
                .push(
                    widget::button::text("Save")
                        .on_press(Message::SaveNewStation)
                )
                .push(
                    widget::button::text("Cancel")
                        .on_press(Message::CancelAddStation)
                )
        );

        self.core.applet.popup_container(form).into()
    }

    /// View for the edit station form
    fn view_edit_station_form(&self, _idx: usize) -> Element<'_, Message> {
        let mut form = widget::column()
            .padding(10)
            .spacing(10);

        // Header
        form = form.push(
            widget::text::text("Edit Station")
                .size(16)
        );

        // Name input
        form = form.push(
            widget::column()
                .spacing(5)
                .push(widget::text::text("Station Name:").size(12))
                .push(
                    widget::text_input("e.g., My Radio Station", &self.edit_station_name)
                        .on_input(Message::EditStationNameChanged)
                )
        );

        // URL input
        form = form.push(
            widget::column()
                .spacing(5)
                .push(widget::text::text("Stream URL:").size(12))
                .push(
                    widget::text_input("e.g., http://example.com/stream.mp3", &self.edit_station_url)
                        .on_input(Message::EditStationUrlChanged)
                )
        );

        // Error message
        if let Some(error) = &self.edit_station_error {
            form = form.push(
                widget::text::text(format!("Error: {}", error))
                    .size(12)
            );
        }

        // Buttons
        form = form.push(
            widget::row()
                .spacing(10)
                .push(
                    widget::button::text("Save")
                        .on_press(Message::SaveEditStation)
                )
                .push(
                    widget::button::text("Cancel")
                        .on_press(Message::CancelEditStation)
                )
        );

        self.core.applet.popup_container(form).into()
    }

    /// View for delete confirmation
    fn view_delete_confirmation(&self, idx: usize) -> Element<'_, Message> {
        let station_name = self.channels.get(idx)
            .map(|c| c.name.as_str())
            .unwrap_or("this station");

        let content = widget::column()
            .padding(10)
            .spacing(10)
            .push(
                widget::text::text("Delete Station?")
                    .size(16)
            )
            .push(
                widget::text::text(format!("Are you sure you want to delete '{}'", station_name))
                    .size(12)
            )
            .push(
                widget::row()
                    .spacing(10)
                    .push(
                        widget::button::text("Delete")
                            .on_press(Message::ConfirmDeleteStation)
                    )
                    .push(
                        widget::button::text("Cancel")
                            .on_press(Message::CancelDeleteStation)
                    )
            );

        self.core.applet.popup_container(content).into()
    }

    /// View for the channel list
    fn view_channel_list(&self) -> Element<'_, Message> {
        // Build the channel list
        let mut content_list = widget::column()
            .padding(5)
            .spacing(2);

        // Add header with current status
        let header_text = if let Some(idx) = self.current_channel_idx {
            if let Some(channel) = self.channels.get(idx) {
                format!("Now Playing: {}", channel.name)
            } else {
                "Internet Radio".to_string()
            }
        } else {
            "Internet Radio".to_string()
        };

        content_list = content_list.push(
            widget::text::text(header_text)
                .size(16)
        );

        // Add stop button if playing
        if self.play_state == State::Playing {
            content_list = content_list.push(
                widget::settings::item(
                    "Stop Playback",
                    widget::button::icon(widget::icon::from_name("media-playback-stop-symbolic"))
                        .on_press(Message::StopPlayback),
                )
            );
        }

        // Add separator
        content_list = content_list.push(widget::divider::horizontal::default());

        // Add each channel
        for (idx, channel) in self.channels.iter().enumerate() {
            let is_playing = self.current_channel_idx == Some(idx) 
                && self.play_state == State::Playing;
            
            let icon_name = if is_playing {
                "media-playback-stop-symbolic"
            } else {
                "media-playback-start-symbolic"
            };

            // Main row with channel name and play button
            let mut row = widget::row()
                .spacing(5)
                .align_y(cosmic::iced::Alignment::Center);

            // Channel name (expand to fill)
            row = row.push(
                widget::text::text(&channel.name)
                    .width(cosmic::iced::Length::Fill)
            );

            // Play/Stop button
            row = row.push(
                widget::button::icon(widget::icon::from_name(icon_name))
                    .on_press(if is_playing {
                        Message::StopPlayback
                    } else {
                        Message::PlayChannel(idx)
                    })
            );

            // Edit button
            row = row.push(
                widget::button::icon(widget::icon::from_name("edit-symbolic"))
                    .on_press(Message::StartEditStation(idx))
            );

            // Delete button
            row = row.push(
                widget::button::icon(widget::icon::from_name("edit-delete-symbolic"))
                    .on_press(Message::StartDeleteStation(idx))
            );

            content_list = content_list.push(row);
        }

        // Add separator before Add Station button
        content_list = content_list.push(widget::divider::horizontal::default());

        // Add Station button
        content_list = content_list.push(
            widget::button::text("+ Add Station")
                .on_press(Message::ToggleAddStation)
        );

        self.core.applet.popup_container(content_list).into()
    }
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

        // Load channels asynchronously
        let load_channels_task = Task::perform(
            async { channels::load_channels() },
            |result| match result {
                Ok(list) => Message::ChannelsLoaded(list.channels),
                Err(e) => Message::ChannelError(e.to_string()),
            },
        ).map(|msg| cosmic::Action::App(msg));

        (app, load_channels_task)
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

        let icon = if self.play_state == State::Playing {
            "audio-card-symbolic"
        } else {
            "audio-card-symbolic"
        };

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

        // Show error message if there is one
        if let Some(error) = &self.error_message {
            let error_widget = widget::column()
                .padding(10)
                .spacing(10)
                .push(widget::text::text("Error loading channels:").size(14))
                .push(widget::text::text(error).size(12))
                .push(
                    widget::button::text("Use Defaults")
                        .on_press(Message::ChannelsLoaded(channels::default_channels().channels))
                );
            return self.core.applet.popup_container(error_widget).into();
        }

        // Show edit station form
        if let Some(idx) = self.editing_station_idx {
            return self.view_edit_station_form(idx);
        }

        // Show delete confirmation
        if let Some(idx) = self.deleting_station_idx {
            return self.view_delete_confirmation(idx);
        }

        // Show add station form
        if self.adding_station {
            return self.view_add_station_form();
        }

        // Show message if no channels loaded yet
        if self.channels.is_empty() {
            let loading_widget = widget::column()
                .padding(10)
                .spacing(10)
                .push(widget::text::text("Loading channels..."))
                .push(
                    widget::button::text("Add Station")
                        .on_press(Message::ToggleAddStation)
                );
            return self.core.applet.popup_container(loading_widget).into();
        }

        self.view_channel_list()
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
                // Legacy toggle - stops if playing, otherwise no-op
                if let Some(player) = &self.player {
                    if self.play_state == State::Playing {
                        if let Err(e) = player.stop() {
                            tracing::error!("Failed to stop playback: {}", e);
                        }
                        self.current_channel_idx = None;
                    }
                }
            },
            Message::PlayChannel(idx) => {
                if let Some(channel) = self.channels.get(idx) {
                    if let Some(player) = &self.player {
                        // Stop any current playback
                        if let Err(e) = player.stop() {
                            tracing::error!("Failed to stop previous playback: {}", e);
                        }
                        
                        // Start playing the selected channel
                        if let Err(e) = player.play(&channel.uri) {
                            tracing::error!("Failed to start playback of {}: {}", channel.name, e);
                            self.error_message = Some(format!("Failed to play {}", channel.name));
                        } else {
                            self.current_channel_idx = Some(idx);
                            self.error_message = None;
                            tracing::info!("Started playing: {} ({})", channel.name, channel.uri);
                        }
                    }
                }
            }
            Message::StopPlayback => {
                if let Some(player) = &self.player {
                    if let Err(e) = player.stop() {
                        tracing::error!("Failed to stop playback: {}", e);
                    }
                }
                self.current_channel_idx = None;
            }
            Message::PlayerStateChanged(state) => {
                self.play_state = state;
                // If playback stops unexpectedly, clear current channel
                if state == State::Null {
                    self.current_channel_idx = None;
                }
            }
            Message::MetadataUpdated(_tags) => {
                // Placeholder for metadata extraction
            }
            Message::ChannelsLoaded(channels) => {
                self.channels = channels;
                self.error_message = None;
                tracing::info!("Loaded {} channels", self.channels.len());
            }
            Message::ChannelError(error) => {
                tracing::error!("Failed to load channels: {}", error);
                self.error_message = Some(error);
            }
            Message::ToggleAddStation => {
                self.adding_station = !self.adding_station;
                if !self.adding_station {
                    // Clear form when closing
                    self.new_station_name.clear();
                    self.new_station_url.clear();
                    self.new_station_error = None;
                }
            }
            Message::NewStationNameChanged(name) => {
                self.new_station_name = name;
                self.new_station_error = None;
            }
            Message::NewStationUrlChanged(url) => {
                self.new_station_url = url;
                self.new_station_error = None;
            }
            Message::SaveNewStation => {
                // Validate inputs
                let name = self.new_station_name.trim();
                let url = self.new_station_url.trim();
                
                if name.is_empty() {
                    self.new_station_error = Some("Station name is required".to_string());
                    return Task::none();
                }
                
                if url.is_empty() {
                    self.new_station_error = Some("Stream URL is required".to_string());
                    return Task::none();
                }
                
                // Basic URL validation
                if !url.starts_with("http://") && !url.starts_with("https://") {
                    self.new_station_error = Some("URL must start with http:// or https://".to_string());
                    return Task::none();
                }
                
                // Generate ID from name
                let id = name.to_lowercase()
                    .replace(' ', "-")
                    .replace(|c: char| !c.is_alphanumeric() && c != '-', "");
                
                if id.is_empty() {
                    self.new_station_error = Some("Invalid station name".to_string());
                    return Task::none();
                }
                
                // Create new channel
                let new_channel = Channel {
                    id,
                    name: name.to_string(),
                    uri: url.to_string(),
                    favourite: false,
                };
                
                // Add to list
                self.channels.push(new_channel);
                
                // Save to file
                let list = ChannelList {
                    channels: self.channels.clone(),
                };
                
                if let Err(e) = channels::save_channels(&list) {
                    tracing::error!("Failed to save channels: {}", e);
                    self.error_message = Some(format!("Failed to save: {}", e));
                    // Remove the channel we just added
                    self.channels.pop();
                } else {
                    tracing::info!("Added new station: {}", name);
                    // Clear form and close
                    self.new_station_name.clear();
                    self.new_station_url.clear();
                    self.new_station_error = None;
                    self.adding_station = false;
                }
            }
            Message::CancelAddStation => {
                self.adding_station = false;
                self.new_station_name.clear();
                self.new_station_url.clear();
                self.new_station_error = None;
            }
            Message::StartEditStation(idx) => {
                if let Some(channel) = self.channels.get(idx) {
                    self.editing_station_idx = Some(idx);
                    self.edit_station_name = channel.name.clone();
                    self.edit_station_url = channel.uri.clone();
                    self.edit_station_error = None;
                }
            }
            Message::EditStationNameChanged(name) => {
                self.edit_station_name = name;
                self.edit_station_error = None;
            }
            Message::EditStationUrlChanged(url) => {
                self.edit_station_url = url;
                self.edit_station_error = None;
            }
            Message::SaveEditStation => {
                if let Some(idx) = self.editing_station_idx {
                    // Validate inputs
                    let name = self.edit_station_name.trim();
                    let url = self.edit_station_url.trim();
                    
                    if name.is_empty() {
                        self.edit_station_error = Some("Station name is required".to_string());
                        return Task::none();
                    }
                    
                    if url.is_empty() {
                        self.edit_station_error = Some("Stream URL is required".to_string());
                        return Task::none();
                    }
                    
                    // Basic URL validation
                    if !url.starts_with("http://") && !url.starts_with("https://") {
                        self.edit_station_error = Some("URL must start with http:// or https://".to_string());
                        return Task::none();
                    }
                    
                    // Update the channel
                    if let Some(channel) = self.channels.get_mut(idx) {
                        let old_id = channel.id.clone();
                        channel.name = name.to_string();
                        channel.uri = url.to_string();
                        // Only regenerate ID if name changed significantly
                        if name.to_lowercase().replace(' ', "-") != old_id {
                            channel.id = name.to_lowercase()
                                .replace(' ', "-")
                                .replace(|c: char| !c.is_alphanumeric() && c != '-', "");
                        }
                        
                        // Save to file
                        let list = ChannelList {
                            channels: self.channels.clone(),
                        };
                        
                        if let Err(e) = channels::save_channels(&list) {
                            tracing::error!("Failed to save channels: {}", e);
                            self.error_message = Some(format!("Failed to save: {}", e));
                        } else {
                            tracing::info!("Updated station: {}", name);
                            // Clear form and close
                            self.editing_station_idx = None;
                            self.edit_station_name.clear();
                            self.edit_station_url.clear();
                            self.edit_station_error = None;
                            
                            // If this was the currently playing channel, stop playback
                            if self.current_channel_idx == Some(idx) {
                                if let Some(player) = &self.player {
                                    let _ = player.stop();
                                }
                                self.current_channel_idx = None;
                            }
                        }
                    }
                }
            }
            Message::CancelEditStation => {
                self.editing_station_idx = None;
                self.edit_station_name.clear();
                self.edit_station_url.clear();
                self.edit_station_error = None;
            }
            Message::StartDeleteStation(idx) => {
                self.deleting_station_idx = Some(idx);
            }
            Message::ConfirmDeleteStation => {
                if let Some(idx) = self.deleting_station_idx {
                    // Remove the channel
                    if idx < self.channels.len() {
                        let removed_channel = self.channels.remove(idx);
                        
                        // Save to file
                        let list = ChannelList {
                            channels: self.channels.clone(),
                        };
                        
                        if let Err(e) = channels::save_channels(&list) {
                            tracing::error!("Failed to save channels after deletion: {}", e);
                            self.error_message = Some(format!("Failed to save: {}", e));
                            // Restore the channel
                            self.channels.insert(idx, removed_channel);
                        } else {
                            tracing::info!("Deleted station: {}", removed_channel.name);
                            
                            // If this was the currently playing channel, stop playback
                            if self.current_channel_idx == Some(idx) {
                                if let Some(player) = &self.player {
                                    let _ = player.stop();
                                }
                                self.current_channel_idx = None;
                            } else if let Some(current_idx) = self.current_channel_idx {
                                // Adjust current channel index if needed
                                if current_idx > idx {
                                    self.current_channel_idx = Some(current_idx - 1);
                                }
                            }
                        }
                    }
                    self.deleting_station_idx = None;
                }
            }
            Message::CancelDeleteStation => {
                self.deleting_station_idx = None;
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
