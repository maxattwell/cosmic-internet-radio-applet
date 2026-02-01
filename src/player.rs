// SPDX-License-Identifier: MPL-2.0

use gstreamer::prelude::*;
use gstreamer::{Pipeline, State};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlayerError {
    #[error("Failed to create element: {0}")]
    CreateElement(#[from] gstreamer::glib::BoolError),
    #[error("Element is not a pipeline")]
    NotAPipeline,
    #[error("State change failed")]
    StateChange,
}

/// A wrapper around a GStreamer pipeline for audio playback.
pub struct Player {
    pipeline: Pipeline,
}

impl Player {
    /// Create a new Player instance.
    ///
    /// This initializes a `playbin3` pipeline.
    pub fn new() -> Result<Self, PlayerError> {
        // Create a playbin3 element
        let playbin = gstreamer::ElementFactory::make("playbin3")
            .build()
            .map_err(PlayerError::CreateElement)?;

        let pipeline = playbin
            .downcast::<Pipeline>()
            .map_err(|_| PlayerError::NotAPipeline)?;

        Ok(Self { pipeline })
    }

    /// Start playback of the given URI.
    pub fn play(&self, uri: &str) -> Result<(), PlayerError> {
        self.pipeline
            .set_state(State::Null)
            .map_err(|_| PlayerError::StateChange)?;
        self.pipeline.set_property("uri", uri);
        self.pipeline
            .set_state(State::Playing)
            .map_err(|_| PlayerError::StateChange)?;
        Ok(())
    }

    /// Stop playback.
    pub fn stop(&self) -> Result<(), PlayerError> {
        self.pipeline
            .set_state(State::Null)
            .map_err(|_| PlayerError::StateChange)?;
        Ok(())
    }

    /// Pause playback.
    pub fn pause(&self) -> Result<(), PlayerError> {
        self.pipeline
            .set_state(State::Paused)
            .map_err(|_| PlayerError::StateChange)?;
        Ok(())
    }

    pub fn set_volume(&self, volume: f64) {
        self.pipeline.set_property("volume", volume);
    }

    pub fn volume(&self) -> f64 {
        self.pipeline.property("volume")
    }

    pub fn bus(&self) -> gstreamer::Bus {
        self.pipeline.bus().expect("Pipeline has no bus")
    }

    pub fn pipeline(&self) -> &Pipeline {
        &self.pipeline
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(State::Null);
    }
}
