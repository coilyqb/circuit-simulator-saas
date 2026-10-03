# Circuit Simulator SaaS

A comprehensive web-based platform for designing analog circuits and running SPICE simulations with real-time waveform visualization.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        CLIENT (React + TS)                       │
│  ├─ Circuit Schematic Editor (Canvas-based)                     │
│  ├─ Component Library (R, L, C, BJT, CMOS, Diode, etc.)        │
│  ├─ Simulation Configuration Panel                              │
│  ├─ Waveform Viewer                                             │
│  └─ Job Status Tracker                                          │
└──────────────────────────┬──────────────────────────────────────┘
                           │ REST API + WebSocket
┌──────────────────────────▼──────────────────────────────────────┐
│                   BACKEND (Rust - Actix-web)                     │
│  ├─ REST API Server (Circuit Management, Simulations)           │
│  ├─ WebSocket Server (Real-time Status Updates)                 │
│  ├─ Job Queue Manager (Redis/In-memory)                         │
│  ├─ Database Layer (MySQL)                                      │
│  └─ Authentication & Authorization                              │
└──────────────────────────┬──────────────────────────────────────┘
                           │ Job Queue (Redis/Channels)
┌──────────────────────────▼──────────────────────────────────────┐
│            PROCESSING SERVERS (Rust + Ngspice)                   │
│  ├─ Job Worker Pool                                             │
│  ├─ SPICE Netlist Generator                                     │
│  ├─ Ngspice Runner                                              │
│  ├─ Result Parser & Storage                                     │
│  └─ Measurement Calculator                                      │
└──────────────────────────┬──────────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────────┐
│                    DATABASE (MySQL)                              │
│  ├─ Circuits (schema, components, connections)                  │
│  ├─ Simulations (config, parameters, status)                    │
│  ├─ Results (waveform data, measurements)                       │
│  ├─ Users & Auth                                                │
│  └─ Audit Logs                                                  │
└──────────────────────────────────────────────────────────────────┘
```

## Stack

- **Frontend**: React 18+, TypeScript, Vite, Canvas API, WebSocket
- **Backend**: Rust, Actix-web, Tokio, SQLx
- **Database**: MySQL 8.0+
- **Processing**: Ngspice, Job Queue
- **DevOps**: Docker, Docker Compose

## Project Structure

```
circuit-simulator-saas/
├── frontend/                 # React + TypeScript client
├── backend/                  # Rust API server
├── worker/                   # Ngspice processing server
├── docker/                   # Docker configurations
├── db/                       # Database schemas & migrations
└── docs/                     # Documentation
```

## Getting Started

See individual README files in each directory for setup instructions.

### Frontend core — Milestone M1

From `frontend/`, run `npm ci`, `npm test`, and `npm run build`.
The build produces an ES module library, not an editor UI.

`frontend/src/core/` is pure TypeScript, usable in Node and Web Workers:

- `model`: JSON document types, structural validation, serialization, and chained
  schema migrations. Version numbers belong outside circuit content; symbol and
  subcircuit references pin their respective versions.
- `symbols`: deeply frozen, versioned built-ins, SPICE pin ordering, and derived
  pin geometry. Only ground maps to global net `0`. Transistor/diode geometry is
  provisional; rendering and SPICE generation are not implemented.
- `commands`: document edits and per-document Immer patch history. Patches are
  internal; callers use `DocumentHistory.execute`, `undo`, and `redo`.
- `utils`: Nano ID allocation and consistent selection paste/document cloning.

No React, UI, or DOM imports belong in core. `tsconfig.core.json` excludes DOM
and ambient browser types; the boundary test restricts production imports to
core-relative files, Immer, and Nano ID. Selection and viewport stay outside the
document and history. Nets, electrical validation, routing, and netlists are
later milestones, not persisted fields.

Create a document with `createDocument()` and wrap it in `DocumentHistory`.
An add command returns its allocated ID. Read `history.document` for the current
deeply frozen snapshot. Component parameters retain raw strings; references are
auto-assigned from the symbol's SPICE prefix and unique case-insensitively.
`beginTransaction()` groups edits until `commitTransaction()` into one undo
step; `cancelTransaction()` restores the starting document and preserves redo.
Nested transactions and undo/redo during a transaction are rejected.

**Deletion policy:** removing a component or junction deletes attached wires.
Deleting a component also deletes pin-attached labels; deleting a wire deletes
wire-attached labels. Each cascade is one undoable command.

**Paste policy:** `history.paste(source, selection, { offset })` allocates new IDs
for every selected element and remaps internal references as one undo step.
External wire endpoints become free at their computed positions; labels attached
to unselected components/wires are dropped. Free endpoints, waypoints, points,
and placements translate together. Paste requires matching grid units.
External subcircuit pins need `resolveExternalPin`; their internal pin IDs belong
to the pinned definition and are not remapped. Pasted references are reassigned
without collisions, and port orders are advanced if occupied. Ports can only be
pasted into subcircuits. `cloneDocument` remaps a whole document, including its
definition ID, while retaining pinned external definition references and metadata.

Positions are document coordinates, integer multiples of `grid`; symbol-local
positions are grid units. Horizontal mirror reflects local X before rotation.
History retains claimed IDs across deletion, undo, and canceled gestures;
this registry is session-local, not saved content. Fresh Nano IDs avoid intentional
reuse after reload. Load through `loadDocument` to upgrade old JSON; each migration
advances one schema version, and missing/newer versions are rejected.

## Features

### Circuit Design
- Draw analog circuits with drag-and-drop components
- Support for R, L, C, BJT, CMOS, Diodes, Voltage/Current sources
- Wire connections with automatic routing
- Pan and zoom capabilities
- Undo/redo functionality

### Simulation
- Configure simulation parameters (PVT - Process, Voltage, Temperature)
- Multiple analysis types:
  - DC Operating Point
  - Transient Analysis
  - AC Analysis
  - DC Sweep
  - Parametric Sweep
- Batch simulation support

### Results & Visualization
- Real-time waveform plotting
- Multiple trace overlay
- Measurement tools (peak, valley, rise time, etc.)
- Export data (CSV, JSON)
- Simulation history tracking

### Backend Features
- User authentication & authorization
- Circuit versioning & collaboration
- Job queue with multiple workers
- Real-time job status via WebSocket
- Comprehensive logging & monitoring

## License

MIT
