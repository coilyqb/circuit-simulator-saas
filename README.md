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
