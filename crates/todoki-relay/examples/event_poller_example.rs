/// Example: How to use Event Poller in a relay-side agent
///
/// This demonstrates how an agent can poll events from the server.
/// Note: In most cases, the Orchestrator will push events to agents automatically.
/// This polling mechanism is mainly for:
/// - Standalone agents
/// - Checking for missed events during reconnection
/// - Historical event queries

use todoki_relay::event_poller::{Event, EventPoller};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();

    // Configuration
    let agent_id = std::env::var("AGENT_ID").unwrap_or_else(|_| "example-agent".to_string());
    let server_url = std::env::var("SERVER_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let token = std::env::var("USER_TOKEN").expect("USER_TOKEN must be set");

    // Events to subscribe to
    let kinds = vec![
        "task.created".to_string(),
        "agent.*".to_string(), // Wildcard: all agent events
    ];

    println!("🔍 Starting Event Poller for agent: {}", agent_id);
    println!("📡 Server: {}", server_url);
    println!("📋 Subscribed event kinds: {:?}", kinds);

    // Create poller
    let poller = EventPoller::new(
        agent_id.clone(),
        server_url,
        token,
        kinds,
    );

    // Initialize cursor from server
    println!("⏳ Initializing cursor...");
    poller.init_cursor().await?;
    println!("✅ Cursor initialized");

    // Example 1: Poll once
    println!("\n📥 Polling once for events...");
    match poller.poll_once().await {
        Ok(events) => {
            if events.is_empty() {
                println!("   No new events");
            } else {
                println!("   Received {} events:", events.len());
                for event in events {
                    println!("   - [{}] {} (cursor: {})", event.agent_id, event.kind, event.cursor);
                }
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    // Example 2: Start continuous polling (background task)
    println!("\n🔄 Starting continuous polling (every 5 seconds)...");
    println!("   Press Ctrl+C to stop");

    poller.start_polling(5, |event: Event| {
        println!("📨 Event received:");
        println!("   Kind: {}", event.kind);
        println!("   Cursor: {}", event.cursor);
        println!("   Agent ID: {}", event.agent_id);
        println!("   Data: {}", event.data);

        // Process event here
        // For example, trigger some action based on event kind
        match event.kind.as_str() {
            "task.created" => {
                println!("   ➡️  Processing new task...");
                // Handle task creation
            }
            kind if kind.starts_with("agent.") => {
                println!("   ➡️  Processing agent event...");
                // Handle agent event
            }
            _ => {
                println!("   ➡️  Ignoring event");
            }
        }
    }).await;

    // Wait indefinitely (in real scenarios, this would be your main loop)
    tokio::signal::ctrl_c().await?;
    println!("\n👋 Shutting down...");

    Ok(())
}
