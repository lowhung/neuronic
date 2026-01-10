# Neuronic - Development Context

## Project Overview

Neuronic is a **real-time graphical visualization tool** for message bus systems, specifically designed to monitor and visualize the Caryatid framework (used in the Cardano node Acropolis). It provides an interactive GUI that displays module connectivity, message flow, and health status through a force-directed graph with animated effects.

**Key Purpose**: Help developers understand system topology, identify bottlenecks, and spot performance issues in distributed message-driven architectures through visual inspection.

## Quick Start

### Build
```bash
cargo build --release
```

### Run
```bash
# Using default config (localhost RabbitMQ, config.default.toml)
cargo run -- --debug

# With custom config file
cargo run -- --config neuronic.toml

# With custom topic
cargo run -- --topic caryatid.monitor.snapshot --debug
```

### Installation
```bash
cargo install --path .
neuronic --config neuronic.toml --debug
```

### Test
```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
```

## Architecture Overview

### Core Data Flow
```
RabbitMQ → Subscriber (lapin) → Snapshot Channel
                                      ↓
                            MessageFlowGraph
                                      ↓
                            GUI (eframe/egui)
                                      ↓
                         Rendering + Animation
```

### Module Structure

The project is organized into 5 main layers:

1. **Config Layer** (`src/config.rs`)
   - TOML-based configuration loading with environment variable overrides
   - Health threshold configuration (backlog/pending warnings and critical levels)
   - RabbitMQ connection settings
   - Topic filtering (ignore noisy topics like "cardano.query.*")

2. **Data Model Layer** (`src/graph.rs`)
   - `MessageFlowGraph` - Main graph structure (wraps petgraph DiGraph)
   - `ModuleNode` - Represents modules/services with throughput and health status
   - `TopicEdge` - Represents message topics connecting producers to consumers
   - `HealthStatus` - Enum: Healthy, Warning, Critical
   - `HealthConfig` - Configurable thresholds for health determination

3. **Connection Layer** (`src/subscriber.rs`)
   - RabbitMQ subscription using lapin client
   - Creates temporary exclusive queues bound to the caryatid exchange
   - Deserializes CBOR-encoded buswatch-types Snapshots
   - Streams snapshots through tokio async runtime to UI

4. **UI State Layer** (`src/ui/state/`)
   - **ConnectionState** - Data flow, RabbitMQ connection, flow graph
   - **ViewState** - Node positions, velocities, camera zoom/pan, layout mode
   - **InteractionState** - Selection, dragging, search box
   - **AnimationState** - Fire intensity, synapse particles, pulse rings
   - **FilterState** - Topic filters, node grouping
   - **UIPreferences** - Theme, visibility toggles, display options

5. **Rendering & Animation Layer** (`src/ui/`)
   - `app.rs` - Main eframe application, event loop integration
   - `drawing.rs` - Graph rendering (nodes, edges, particles, minimap)
   - `layout.rs` - Force-directed and hierarchical layout algorithms
   - `animations.rs` - Particle effects, pulse rings, fire intensity
   - `input.rs` - Mouse/keyboard handling
   - `panels.rs` - Side panels (legend, filters, node/edge details)
   - `search.rs` - Fuzzy search (Ctrl+F)
   - `export.rs` - SVG export
   - `theme.rs` - Light/dark color schemes
   - `types.rs` - Shared rendering types

### Key Abstractions & Patterns

#### 1. State Decomposition
The monolithic app state is split into focused sub-structs in `ui/state/`:
- Each struct handles one domain (view, connection, animation, etc.)
- Makes code easier to understand, test, and modify
- Recent refactor: `51459a6` extracted these from monolithic NeuronicApp

#### 2. Health System
Three-level health status based on configurable thresholds:
- **Healthy**: All metrics below warning levels
- **Warning**: Backlog or pending time exceeds warning threshold
- **Critical**: Backlog or pending time exceeds critical threshold

Health is computed from:
- Per-topic backlog (unread message count)
- Per-topic pending time (age of oldest unread message)

#### 3. Layout Algorithms
Two layout modes with physics-based positioning:
- **Force-directed** (default): Nodes repel each other, edges attract. Self-organizes into natural clusters.
  - Repulsion: 50000.0 (configurable)
  - Attraction: 0.002
  - Damping: 0.85
  - Minimum separation: 120px
  
- **Hierarchical**: Sources at top, sinks at bottom. Better for understanding dataflow direction.

#### 4. Bezier Curve Edges
- All edges use quadratic Bezier curves (control point between start/end)
- Provides clear visual separation between parallel edges
- Curves are sampled into line segments for rendering

#### 5. Animation System
Three types of visual effects triggered by message activity:
- **Node Fire**: Intensity increases when messages flow, decays over time (2.0 decay rate)
- **Synapse Particles**: Flow along edges showing message movement
- **Pulse Rings**: Expand from nodes during heavy activity (50+ message burst), fade and disappear

#### 6. Topic Filtering
- Prefix-based topic matching (e.g., "cardano.query." hides all cardano.query.* topics)
- Applied during graph update to reduce noise and improve performance
- Configurable via config file or environment variables

## Configuration System

### Sources (in priority order):
1. `config.default.toml` (shipped defaults)
2. User config file (default: `neuronic.toml`, or via `--config`)
3. Environment variables with `NEURONIC_` prefix (e.g., `NEURONIC_GRAPH_BACKLOG_WARNING=50`)

### Example Configuration (`config.default.toml`)
```toml
# RabbitMQ connection
[rabbitmq]
url = "amqp://127.0.0.1:5672/%2f"
exchange = "caryatid"

# Topic filtering - hide noisy topics
[filter]
ignored_topics = ["cardano.query."]

# Health thresholds
[graph]
backlog_warning = 1000
backlog_critical = 10000
pending_warning_ms = 5000
pending_critical_ms = 20000
```

### Configuration Precedence
Later sources override earlier ones. Environment variables are most powerful.

## Key Files and Responsibilities

| File | Lines | Purpose |
|------|-------|---------|
| `src/lib.rs` | 47 | Library entry point, public API |
| `src/main.rs` | 58 | CLI entry point, argument parsing, window setup |
| `src/config.rs` | 281 | TOML loading, environment variable integration |
| `src/graph.rs` | 445 | Graph model, snapshot processing, health calculation |
| `src/subscriber.rs` | 112 | RabbitMQ connection, snapshot deserialization |
| `src/ui/app.rs` | 468 | Main application loop, frame updates |
| `src/ui/layout.rs` | 545 | Force-directed and hierarchical layout |
| `src/ui/drawing.rs` | 814 | Graph rendering (nodes, edges, effects) |
| `src/ui/animations.rs` | 464 | Animation state updates |
| `src/ui/panels.rs` | 449 | UI panels (legend, details, filters) |
| `src/ui/theme.rs` | 224 | Color definitions for light/dark themes |
| `src/ui/search.rs` | 340 | Fuzzy search implementation |
| `src/ui/input.rs` | 155 | Mouse and keyboard handling |
| `src/ui/export.rs` | 407 | SVG export functionality |
| **Total** | **5120** | |

## Dependencies Overview

### Core Visualization
- **eframe/egui** (0.33) - Cross-platform GUI framework
- **petgraph** (0.8) - Graph data structures and algorithms
- **egui_graphs** (0.29) - Graph-specific UI components

### Message Bus Integration
- **lapin** (3) - RabbitMQ AMQP client
- **buswatch-types** (0.1) - Snapshot format from buswatch/Caryatid
- **minicbor-serde** (0.6) - CBOR deserialization

### Async Runtime
- **tokio** (1, full features) - Async runtime for connection handling

### Configuration & CLI
- **config** (0.15) - TOML parsing and environment variable integration
- **clap** (4, derive) - Command-line argument parsing
- **serde** / **serde_json** (1) - Serialization

### Utilities
- **tracing** (0.1) - Structured logging
- **anyhow** / **thiserror** (1) - Error handling
- **rfd** (0.16) - File dialogs
- **futures-util** (0.3) - Async utilities

**Minimum Rust Version**: 1.85

## Entry Points

### Binary
`src/main.rs` → CLI argument parsing → `NeuronicApp::new()` → eframe event loop

### Key Initialization Flow
1. Parse CLI args (config path, topic, debug flag)
2. Load NeuronicConfig from files + environment
3. Create MessageFlowGraph with health config
4. Connect to RabbitMQ (async in tokio runtime)
5. Initialize UI state (view, animation, filters, preferences)
6. Start eframe GUI loop

### GUI Frame Loop
```rust
fn update(&mut self, ctx: &egui::Context) {
    // 1. Check for new snapshots from RabbitMQ
    // 2. Update flow_graph from snapshot
    // 3. Run layout algorithm
    // 4. Detect animation activity
    // 5. Handle user input (mouse, keyboard)
    // 6. Render graph, panels, effects
    // 7. Request next frame if needed
}
```

## Recent Changes & Development

### Current Branch: `infer-synapse-rate`
- Work in progress for inferring synapse (edge) message rates from message count deltas

### Recent Commits
- `6868250` - feat: add snapshot capture API for external tools
- `1f0ff87` - refactor: extract RateTracker with exponential smoothing
- `f220224` - feat: infer synapse rate from message count delta
- `f87b6a0` - Merge PR #48: decompose NeuronicApp into sub-structs
- `51459a6` - refactor: decompose NeuronicApp into sub-structs (major refactoring)

### Known Structure
- Modified files on current branch:
  - `config.default.toml` - Configuration changes
  - `src/subscriber.rs` - Connection handling improvements

## Testing

### Test Coverage
- `src/graph.rs` - Unit tests for health detection and graph updates
- `src/config.rs` - Configuration loading tests
- No integration tests (would require RabbitMQ)

### Running Tests
```bash
cargo test                    # Run all tests
cargo test --lib             # Library tests only
cargo test -- --nocapture    # Show output
```

## Important Patterns & Conventions

### Health Calculation
Modules and edges are marked critical/warning/healthy based on the worst metric:
```rust
fn compute_module_health(metrics) {
    for (topic, read_metrics) in reads {
        if backlog >= critical_threshold { return Critical; }
        if backlog >= warning_threshold { worst = Warning; }
    }
    for (topic, write_metrics) in writes {
        if pending_us >= critical_threshold { return Critical; }
        if pending_us >= warning_threshold { worst = Warning; }
    }
}
```

### Topic Filtering
Filters are applied during snapshot processing to hide ignored topics:
- Doesn't remove nodes, just prevents edge/topic creation
- Re-filtering happens on each snapshot update

### Snapshot Processing
1. First pass: Create/update module nodes from snapshot metrics
2. Clear all edges (rebuild each time)
3. Second pass: Create edges from producers to consumers
4. Compute health status for all nodes and edges

## Related Projects

- **buswatch** - TUI alternative, same data source
- **Caryatid** - The framework that publishes monitor snapshots
- **Acropolis** - Cardano node implementation using Caryatid

## Common Development Tasks

### Adding a New Configuration Option
1. Add field to appropriate struct in `src/config.rs`
2. Implement `Default` impl
3. Update `config.default.toml`
4. Use in application logic

### Adding a New Visual Effect
1. Define animation data structure in `src/ui/types.rs`
2. Implement update logic in `src/ui/animations.rs`
3. Add rendering code in `src/ui/drawing.rs`
4. Hook into frame update in `src/ui/app.rs`

### Debugging
- Run with `--debug` flag for detailed logging via tracing
- Set `RUST_LOG=debug` environment variable for tracing output
- Use `cargo run -- --debug` for development

## Notes for Claude

- State is heavily decomposed into sub-structs in `ui/state/` - understand this architecture first
- Layout algorithms run every frame but use damping to converge naturally
- Snapshot updates are processed sequentially; no concurrent snapshot handling
- Animation effects are purely visual and decoupled from data model
- Health thresholds are critical for end-user experience (tuning is important)
- RabbitMQ connection is blocking on initialization but async during runtime
- Bezier curves are key to visual clarity with many overlapping edges
