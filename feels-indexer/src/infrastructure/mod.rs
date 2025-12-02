//! Infrastructure layer
//!
//! Cross-cutting concerns like configuration, telemetry, and service orchestration

pub mod service_container;

pub use service_container::ServiceContainer;

