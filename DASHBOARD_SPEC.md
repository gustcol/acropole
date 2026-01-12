# Integrity Monitoring Dashboard Specification

## 1. Overview

The Integrity Monitoring Dashboard is a modern, responsive web application designed to visualize the real-time status of all agents running the Golden Image Integrity System. It provides a centralized view of fleet health, active alerts, and historical data.

## 2. Architecture

```mermaid
flowchart LR
    subgraph Agents
        Agent1[VM Agent 1]
        Agent2[VM Agent 2]
        AgentN[VM Agent N]
    end

    subgraph Backend [Docker Container]
        API[Metadata Service (Rust)]
        KV[(Sled DB)]
    end

    subgraph Frontend [Docker Container]
        React[React Dashboard]
        Nginx[Nginx Server]
    end

    Agent1 -->|Heartbeat/Alerts| API
    Agent2 -->|Heartbeat/Alerts| API
    AgentN -->|Heartbeat/Alerts| API

    API <--> KV
    React <-->|Fetch Status| API
```

## 3. Tech Stack

- **Frontend Framework**: React 18+ (Create React App or Vite)
- **UI Library**: Material UI (MUI) or Tailwind CSS + ShadcnUI (for a modern, clean look)
- **State Management**: React Query (TanStack Query) for efficient API data fetching and caching.
- **Routing**: React Router
- **Charts**: Recharts or Chart.js for visualization (e.g., healthy vs. compromised nodes over time).
- **Containerization**: Docker (Multi-stage build).

## 4. UI/UX Design

### 4.1. Dashboard (Home)

- **Summary Cards**:
  - Total Agents Online
  - Healthy Agents (Green)
  - Agents with Warnings (Yellow)
  - Compromised Agents (Red)
- **Live Activity Feed**: A scrolling list of the latest alerts and heartbeats.
- **Cluster Health Graph**: A time-series chart showing the number of active violations over the last 24 hours.

### 4.2. Agent List View

- **Data Grid**: A sortable/filterable table of all known agents.
- **Columns**: Hostname, Image ID, IP Address, Last Seen, Status, Version.
- **Actions**: "View Details", "Quarantine" (if integrated with orchestrator).

### 4.3. Agent Detail View

- **Metadata**: VM details, OS version, Image ID.
- **Integrity Status**: Current state of file verification.
- **Violation History**: A list of all integrity violations reported by this specific agent.
- **Baseline Diff**: A visual representation of modified/added/deleted files compared to the golden image.

## 5. API Requirements (Metadata Service Extension)

To support the dashboard, the `metadata-service` needs the following new endpoints:

- `POST /agents/heartbeat`: Receives heartbeat JSON from agents.
- `POST /agents/alert`: Receives violation alerts.
- `GET /agents`: Returns a list of all registered agents and their latest status.
- `GET /agents/{id}`: Returns details for a specific agent.
- `GET /alerts`: Returns a paginated list of alerts (global or per agent).

## 6. Implementation Plan

### Phase 1: Backend Extension

1. Update `integrity-common` with `Heartbeat` and `Alert` structs.
2. Update `metadata-service` to store agent state in Sled (e.g., using a separate tree `agents`).
3. Implement REST endpoints for the dashboard.

### Phase 2: Frontend Development

1. Initialize React project with TypeScript.
2. Implement API client service.
3. Build "Dashboard" and "Agent List" components.
4. Integrate auto-refresh (polling) for real-time updates.

### Phase 3: Dockerization

1. Create `Dockerfile` for the React app (build -> serve with Nginx).
2. Create `docker-compose.yml` to orchestrate `metadata-service` and `dashboard`.
