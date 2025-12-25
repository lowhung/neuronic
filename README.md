# Neuronic

Real-time graphical visualization of Caryatid message bus flow.

## Overview

Neuronic subscribes to the same RabbitMQ topic as `buswatch` (`caryatid.monitor.snapshot`) and renders an interactive force-directed graph showing:

- **Nodes** = Modules (e.g., `block_unpacker`, `accounts_state`, `epochs_state`)
- **Edges** = Topics that connect them (producer → topic → consumer)
- **Visual encodings** for message rate, backlog, and health status

## Features

- Real-time graph updates as snapshots arrive
- Force-directed layout with automatic clustering
- Bottleneck detection and highlighting
- Interactive zoom, pan, and node selection
- Click nodes to see detailed metrics

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# With default config file (neuronic.toml)
neuronic

# Specify config and topic
neuronic --config /path/to/config.toml --topic caryatid.monitor.snapshot

# Enable debug logging
neuronic --debug
```

## Configuration

Create a `neuronic.toml` file:

```toml
[rabbitmq]
url = "amqp://127.0.0.1:5672/%2f"
exchange = "caryatid"

[graph]
layout = "force-directed"
update_interval_ms = 100
backlog_warning_threshold = 100
backlog_critical_threshold = 1000
pending_warning_ms = 500
pending_critical_ms = 2000

[ui]
theme = "dark"
show_topic_labels = true
animate_messages = true
```

## Visual Legend

| Element | Meaning |
|---------|---------|
| Node size | Total throughput (reads + writes) |
| Node color (green) | Healthy - no backlog or pending |
| Node color (yellow) | Warning - backlog or pending approaching threshold |
| Node color (red) | Critical - backlog or pending exceeded threshold |
| Edge width | Message rate on that topic |
| Edge color | Backlog severity |

## Architecture

```
neuronic/
├── src/
│   ├── main.rs           # Entry point, CLI args
│   ├── subscriber.rs     # RabbitMQ connection
│   ├── graph.rs          # Graph data model (petgraph)
│   └── ui/
│       ├── mod.rs        # UI module exports
│       ├── app.rs        # Main eframe application
│       ├── graph_view.rs # Graph rendering with egui_graphs
│       └── controls.rs   # UI controls, legend, filters
```

## Dependencies

- [eframe](https://github.com/emilk/egui) - Cross-platform GUI framework
- [egui_graphs](https://github.com/blitzarx1/egui_graphs) - Graph widget for egui
- [petgraph](https://docs.rs/petgraph) - Graph data structure
- [lapin](https://docs.rs/lapin) - RabbitMQ AMQP client
- [buswatch-types](../acropolis/buswatch-types) - Snapshot deserialization

## Related

- [buswatch](../acropolis/buswatch-tui) - TUI version for terminal-based monitoring

## License

Apache-2.0
