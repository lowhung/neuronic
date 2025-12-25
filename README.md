# Neuronic

Real-time graph visualization for message bus systems.

![Graph with pulse animations](graph_with_pulse.jpeg)

## Background

Neuronic was developed for [Acropolis](https://github.com/input-output-hk/acropolis), a modular Rust implementation of a Cardano node. Acropolis uses the [Caryatid](https://github.com/input-output-hk/caryatid) framework, which provides an event-driven architecture where modules communicate over a message bus (RabbitMQ or an in-memory bus for single-process deployments).

Caryatid includes a monitoring layer that wraps the message bus and tracks per-module, per-topic metrics: message counts, backlog depths, and pending durations. These snapshots are published periodically to a configurable topic (default: `caryatid.monitor.snapshot`).

[buswatch](https://github.com/input-output-hk/buswatch) provides a TUI for this data. Neuronic provides a GUI with force-directed graph layout, making it easier to understand topology and spot bottlenecks visually.

Both tools depend on `buswatch-types` for snapshot deserialization. The architecture is not Cardano-specific - any system publishing compatible monitoring snapshots can use these tools.

## Overview

Neuronic subscribes to a monitoring topic and renders module connectivity as an interactive graph:

- **Nodes** represent modules/services
- **Edges** represent topics connecting producers to consumers
- **Colors** indicate health status based on configurable thresholds

![Graph without pulse animations](graph_without_pulse.jpeg)

## Features

- **Live updates** - graph redraws as snapshots arrive
- **Force-directed layout** - physics-based node positioning with repulsion/attraction forces
- **Curved Bezier edges** - quadratic curves for clear edge tracing
- **Node dragging** - manual repositioning when needed
- **Light/dark themes**
- **Fuzzy search** (Ctrl+F)
- **Configurable health thresholds** - warning/critical states based on backlog depth or pending time

Visual indicators:
- Particles flow along edges during active message traffic
- Pulse rings expand from nodes under heavy load
- Node intensity scales with throughput

## Installation

```bash
cargo install --path .
neuronic
```

With options:

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

Environment variables are also supported with the `NEURONIC_` prefix.

## Layout modes

- **Force-directed** (default) - nodes repel, edges attract. Organic clustering.
- **Hierarchical** - sources at top, sinks at bottom. Useful for understanding dataflow direction.

## Project structure

```
src/
├── main.rs           # CLI entry point
├── config.rs         # Configuration loading
├── subscriber.rs     # RabbitMQ subscriber
├── graph.rs          # Graph model (petgraph)
└── ui/
    ├── app.rs        # eframe application
    ├── theme.rs      # Color schemes
    ├── drawing.rs    # Bezier edge rendering
    ├── input.rs      # Mouse/keyboard handling
    ├── layout.rs     # Force-directed simulation
    ├── animations.rs # Particle and pulse effects
    └── panels.rs     # Side panels
```

## Dependencies

- [eframe/egui](https://github.com/emilk/egui) - Cross-platform GUI
- [petgraph](https://docs.rs/petgraph) - Graph data structure
- [lapin](https://docs.rs/lapin) - RabbitMQ AMQP client
- [buswatch-types](https://github.com/input-output-hk/buswatch) - Snapshot format

## Related

- [buswatch](https://github.com/input-output-hk/buswatch) - TUI for the same monitoring data
- [Caryatid](https://github.com/input-output-hk/caryatid) - The underlying modular framework
- [Acropolis](https://github.com/input-output-hk/acropolis) - Cardano node implementation using Caryatid

## License

Apache-2.0
