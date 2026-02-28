// Event Bus - Phase 1 Core Module
//
// This module provides the core event-driven architecture for agent collaboration.
// Events are persisted to the database and can be queried, subscribed, and replayed.

pub mod types;
pub mod kinds;
pub mod store;
pub mod publisher;
pub mod subscriber;

pub use types::Event;
pub use kinds::EventKind;
pub use store::{EventStore, PgEventStore};
pub use publisher::EventPublisher;
pub use subscriber::EventSubscriber;
