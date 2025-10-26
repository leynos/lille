//! Behaviour-driven tests for physics and motion rules.
//!
//! These scenarios exercise the DBSP circuit via a headless Bevy app and use
//! `rust-rspec` to express expectations declaratively. Each submodule focuses on
//! a distinct slice of the physics pipeline while sharing the common
//! [`support`] harness.

mod support;
mod heights;
mod forces;
mod friction;
mod health;
