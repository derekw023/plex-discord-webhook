//! This crate provides a basic API to translate between the form submitted by a plex server and
//! types that can be used to interact with the posted data

/// Provides structures and types to represent the data posted by plex
pub mod models;

/// Provides a handler function that can be composed into a warp [warp::Filter]
pub mod webhook;
