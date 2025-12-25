# Neuronic

A real-time graph visualization for message bus systems. Watch your modules talk to each other in a force-directed graph instead of staring at logs.

Built to work with [buswatch](https://github.com/yourusername/buswatch) (which I also wrote) - they share the same snapshot format, so you can use the TUI or the GUI depending on your mood.

![Graph with pulse animations](graph_with_pulse.jpeg)

## What is this?

Neuronic subscribes to a monitoring topic on your message bus and renders what it sees as an interactive graph:

- **Nodes** = your modules/services
- **Edges** = topics/queues connecting them
- **Colors** = health status (green is good, red means go look at it)

It's useful for understanding how data flows through your system, spotting bottlenecks, and figuring out why that one queue keeps backing up.

![Graph without pulse animations](graph_without_pulse.jpeg)

## Features

- **Live updates** as messages flow through the system
- **Force-directed layout** that settles into something readable
- **Curved Bezier edges** so you can actually trace connections
- **Drag nodes around** when the physics doesn't get it right
- **Light/dark themes**
- **Fuzzy search** (Ctrl+F)
- **Health thresholds** - configure when nodes go yellow/red based on backlog or pending time

Visual feedback:
- Particles flowing along edges show active message traffic
- Pulse rings when a node is firing heavily
- Node glow intensity based on throughput

## Getting started

```bash
cargo install --path .
neuronic
```

Or with a config:

```bash
neuronic --config neuronic.toml --debug
```

## Configuration

```toml
[message-bus.external]
class = "rabbit-mq"
url = "amqp://127.0.0.1:5672/%2f"
exchange = "your-exchange"

[filter]
ignored_topics = ["noisy.topic.prefix."]

[graph]
backlog_warning = 100
backlog_critical = 1000
pending_warning_ms = 500
pending_critical_ms = 2000
```

Env vars work too (`NEURONIC_` prefix).

## Layout modes

- **Force-directed** (default) - nodes repel each other, edges attract. Looks organic.
- **Hierarchical** - sources at top, sinks at bottom. Good for understanding data flow.

## Project structure

```
src/
├── main.rs           # CLI
├── config.rs         # Config loading
├── subscriber.rs     # Message bus connection
├── graph.rs          # Graph model (petgraph)
└── ui/
    ├── app.rs        # Main app loop
    ├── theme.rs      # Light/dark themes
    ├── drawing.rs    # Rendering (Bezier edges, nodes)
    ├── input.rs      # Mouse/keyboard
    ├── layout.rs     # Force simulation
    ├── animations.rs # Particles, pulses
    └── panels.rs     # Info panels
```

## Dependencies

- [eframe/egui](https://github.com/emilk/egui) - GUI
- [petgraph](https://docs.rs/petgraph) - Graph structure
- [lapin](https://docs.rs/lapin) - RabbitMQ client
- [buswatch-types](https://github.com/yourusername/buswatch) - Shared snapshot format

## Related

- [buswatch](https://github.com/yourusername/buswatch) - TUI version, same data, terminal-based

## License

Apache-2.0
