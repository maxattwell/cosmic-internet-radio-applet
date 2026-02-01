// SPDX-License-Identifier: MPL-2.0

use gstreamer::prelude::*;
use gstreamer::{Pipeline, State};

pub struct Player {
    pipeline: Pipeline,
}

impl Player {
    pub fn new() -> Self {
        // Create a playbin3 element
        let playbin = gstreamer::ElementFactory::make("playbin3")
            .build()
            .expect("Failed to create playbin3 element");

        let pipeline = playbin
            .downcast::<Pipeline>()
            .expect("playbin3 is not a pipeline");

        Self { pipeline }
    }

    pub fn play(&self, uri: &str) {
        self.pipeline.set_state(State::Null).ok();
        self.pipeline.set_property("uri", uri);
        self.pipeline.set_state(State::Playing).ok();
    }

    pub fn stop(&self) {
        self.pipeline.set_state(State::Null).ok();
    }

    pub fn pause(&self) {
        self.pipeline.set_state(State::Paused).ok();
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

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}
