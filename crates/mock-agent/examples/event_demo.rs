//! Demo: How to send events to mock-agent
//!
//! This example shows how to programmatically send events to mock-agent
//! using ACP's ext_notification method.
//!
//! In the real Todoki system:
//! - The relay receives events from EventOrchestrator
//! - Relay forwards events to agents via ext_notification
//! - Agents process events and emit new events

use serde_json::json;

fn main() {
    println!("Mock Agent Event Handling Demo");
    println!("================================\n");

    println!("In the Todoki system, events flow like this:\n");
    println!("1. Server EventBus stores events");
    println!("2. EventOrchestrator monitors events and triggers subscribed agents");
    println!("3. Relay forwards events to agent via ACP ext_notification:");
    println!("   {{");
    println!("     \"jsonrpc\": \"2.0\",");
    println!("     \"method\": \"event\",");
    println!("     \"params\": {{");
    println!("       \"cursor\": 1,");
    println!("       \"kind\": \"task.created\",");
    println!("       \"agent_id\": \"planner-agent\",");
    println!("       \"task_id\": \"task-123\",");
    println!("       \"data\": {{\"content\": \"Fix bug\"}}");
    println!("     }}");
    println!("   }}\n");

    println!("4. Agent processes event and may emit new events\n");

    // Example event structures
    let task_created = json!({
        "cursor": 1,
        "kind": "task.created",
        "time": "2026-02-27T10:00:00Z",
        "agent_id": "planner-agent",
        "task_id": "task-123",
        "session_id": null,
        "data": {
            "content": "Implement user authentication",
            "priority": "high"
        }
    });

    println!("Example event: task.created");
    println!("{}\n", serde_json::to_string_pretty(&task_created).unwrap());

    let requirement_analyzed = json!({
        "cursor": 2,
        "kind": "agent.requirement_analyzed",
        "time": "2026-02-27T10:00:05Z",
        "agent_id": "planner-agent",
        "task_id": "task-123",
        "session_id": "session-456",
        "data": {
            "plan": "1. Design auth flow, 2. Implement JWT, 3. Add middleware",
            "estimated_effort": "high",
            "breakdown": [
                {"subtask": "Architecture design", "assignee": "architect-agent"},
                {"subtask": "JWT implementation", "assignee": "coding-agent"},
                {"subtask": "Testing", "assignee": "qa-agent"}
            ]
        }
    });

    println!("Example event: agent.requirement_analyzed");
    println!("{}\n", serde_json::to_string_pretty(&requirement_analyzed).unwrap());

    println!("Agent Event Subscriptions:");
    println!("- Planner Agent subscribes to: task.created, task.updated");
    println!("- BA Agent subscribes to: agent.requirement_analyzed");
    println!("- Coding Agent subscribes to: agent.requirement_analyzed, code.review_requested");
    println!("\nWhen an event matches subscription, the agent's ext_notification handler is called.");
}
