# Consolidated Egress Offload Metrics and Cloud Cost Savings

## Status

Accepted for version 0.10.10.

## Context

The central thesis of Ponte Mesh is to demonstrate that a hybrid content distribution model combining centralized Origin authority with peer-to-peer fragment sharing and auxiliary Replica/Edge nodes can significantly alleviate central server bandwidth demand (egress) without sacrificing access control, cryptographic verification, or predictable fallback.

Evaluating this hypothesis requires objective, real-time telemetry. Previously, operators could view raw origin traffic (bytes served and requests) and replica sync metrics, but lacked a unified metric quantifying:
1. Total data demanded by consumers across all distribution channels.
2. The exact offload percentage achieved via P2P transfers and Replica nodes.
3. The monetary impact (cloud egress cost savings) enabled by offloading traffic away from the central Origin server.

Furthermore, autonomous agents operating over MCP and administrative dashboards need direct, structured access to evaluate whether hybrid distribution policies are meeting cost-reduction goals.

## Decision

1. **Consolidated Offload Telemetry**: The Origin provides a dedicated administrative endpoint `GET /api/admin/metrics/offload?period={1h|24h|7d|30d|all}` backed by persistent PostgreSQL events (`origin_traffic_events` and `fragment_transfer_events`).
2. **Standardized Telemetry Model**:
   - `total_bytes_demanded` = `origin_bytes_served + replica_bytes_served + peer_bytes_served`
   - `origin_offload_bytes` = `replica_bytes_served + peer_bytes_served`
   - `offload_ratio_percent` = `(origin_offload_bytes / total_bytes_demanded) * 100.0`
   - `peer_offload_ratio_percent` = `(peer_bytes_served / total_bytes_demanded) * 100.0`
   - `replica_offload_ratio_percent` = `(replica_bytes_served / total_bytes_demanded) * 100.0`
3. **Cloud Egress Cost Model**: Savings are calculated using a standard cloud public egress benchmark rate of US$ 0.08 per gigabyte (`$0.08 * (origin_offload_bytes / 1,073,741,824)`), providing tangible ROI metrics.
4. **First-Class MCP Integration**:
   - Tool `pontemesh_get_offload_metrics`: Enables LLM agents to query offload statistics across configurable time windows.
   - Resource `pontemesh://metrics/offload`: Read-only streaming resource of live offload telemetry.
   - Prompt `analyze_egress_offload`: Guided workflow prompting agents to review offload efficacy, fallback rates, and cloud savings.
5. **Web Console Visualization**: Dedicated dashboard efficiency cards in the web panel displaying the overall Offload Ratio (%), Total Demanded Egress, Estimated USD Savings, and Offloaded Bytes.

## Consequences

- Direct empirical validation for the academic graduation thesis (TCC) quantifying central server bandwidth reduction under hybrid load.
- Real-time visibility into the effectiveness of bucket policies (`allowPeerSharing`, `allowReplicaEdge`, and priority strategies).
- Autonomous AI agents and human operators can monitor bandwidth savings and cost reduction without manually correlating raw database events.
- Zero impact on data path performance: metrics are aggregated asynchronously from existing transfer event records.
