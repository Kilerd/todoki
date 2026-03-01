/// Re-export EventKind from shared protocol
///
/// All event kind constants are now defined in todoki-protocol::event_bus::EventKind.
/// This module re-exports it for backward compatibility within todoki-server.

pub use todoki_protocol::EventKind;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_kinds_format() {
        // Verify all event kinds follow the namespace.action format
        assert!(EventKind::TASK_CREATED.contains('.'));
        assert!(EventKind::AGENT_STARTED.contains('.'));
        assert!(EventKind::ARTIFACT_CREATED.contains('.'));
    }
}
