# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2024-12-25

### Added

- Initial release
- Real-time graph visualization of message bus flow
- Force-directed and hierarchical layout modes
- Curved Bezier edges for clear connection tracing
- Light and dark themes
- Node dragging for manual positioning
- Fuzzy search with Ctrl+F
- Health status visualization (healthy/warning/critical)
- Synapse particle animations showing message flow
- Pulse ring animations for high-activity nodes
- Topic filtering to hide noisy topics
- Node grouping by name patterns
- SVG export
- Minimap view
- RabbitMQ subscriber for Caryatid monitor snapshots
- Configurable health thresholds
- Environment variable configuration support

### Dependencies

- buswatch-types for snapshot format compatibility
- eframe/egui for cross-platform GUI
- petgraph for graph data structure
- lapin for RabbitMQ AMQP client

[Unreleased]: https://github.com/lowhung/neuronic/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/lowhung/neuronic/releases/tag/v0.1.0
