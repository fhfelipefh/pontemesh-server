# Multi-Drive Storage Pools and Hot-Drain Drive Replacement

## Status

Proposed for version 0.11.0.

## Context

In high-throughput server deployments—especially game distribution platforms and binary asset repositories handling multiple terabytes of data—storage management requires flexibility without taking the server offline.

Operators frequently encounter scenarios such as:
1. **Capacity Expansion Without Shutdown**: Adding new physical disks (SATA/SAS/NVMe) to a live server to expand storage capacity immediately.
2. **Defective Disk Replacement (Hot-Swap)**: Replacing a disk exhibiting SMART degradation, bad sectors, or I/O errors before catastrophic failure occurs.
3. **Hardware Storage Upgrades**: Migrating data from smaller mechanical hard drives to larger NVMe solid-state arrays.

Previously, the Ponte Mesh Server assumed a single primary directory path (`storage.local.path` in `instance.toml`). While operating-system-level volume managers (such as LVM, ZFS, or hardware RAID) allow online partition growth that the server's `DiskGuard` dynamically detects via `sysinfo`, relying solely on OS volume managers introduces operational complexity for operators using simple multi-disk servers (JBOD - Just a Bunch of Disks). 

Furthermore, if a single disk fills up to the configured `block_percent` threshold, the server blocks all incoming writes globally, even if the host machine has secondary disks with terabytes of free space available.

## Decision

To support live multi-drive environments without breaking existing deployments or burdening operators with complex setups, the Ponte Mesh Server will implement **Multi-Drive Storage Pools with Hot-Drain Support**:

### 1. Complete Backward Compatibility (Zero Breaking Changes)

Existing configurations with a single storage path remain 100% supported without modifications:

```toml
[storage.local]
path = "/var/lib/pontemesh/storage"
```

Operators wishing to attach multiple storage drives can simply configure additional mount points:

```toml
[storage.local]
path = "/var/lib/pontemesh/storage" # Primary drive (default)
extra_paths = [
    "/mnt/nvme-drive2",
    "/mnt/storage-drive3"
]
allocation_strategy = "MOST_AVAILABLE_FREE_SPACE" # Options: MOST_AVAILABLE_FREE_SPACE, ROUND_ROBIN
```

Alternatively, drives can be added dynamically at runtime via the web administration console or MCP administrative tools without restarting the server.

### 2. Native Exploitation of Catalog Object Storage Paths

The PostgreSQL catalog schema already models each object with an explicit absolute path:

```rust
pub struct ObjectRecord {
    pub key: String,
    pub storage_path: String,
    // ...
}
```

Because reads, S3 Range requests, and manifest fragment generators resolve files directly from `object.storage_path`, objects stored across different physical mount points or drives can be retrieved concurrently without indirection or performance penalties.

### 3. Per-Drive DiskGuard and Resilient Allocation

1. **Independent Drive Health Telemetry**: The server evaluates filesystem usage (`total`, `available`, `used_percent`) and write probes independently for each configured drive.
2. **Graceful Overflow Ingestion**:
   - The default allocation strategy (`MOST_AVAILABLE_FREE_SPACE`) routes incoming uploads to the active drive that possesses the greatest available free space and whose disk usage is below the `block_percent` threshold.
   - If a specific drive becomes full or degraded, the server isolates writes to that drive while continuing to accept uploads on the remaining healthy drives in the pool. Write lockouts only occur if **all** configured drives in the pool exceed the blocking threshold.

### 4. Hot-Drain Protocol for Safe Drive Replacement

To retire or replace a drive without interrupting service:

1. **State Transition to `DRAINING`**:
   - The operator marks a drive as draining via API (`POST /api/admin/storage/drives/{drive_id}/drain`), web UI, or MCP tool (`pontemesh_drain_storage_drive`).
   - The server immediately stops directing new uploads to the target drive.
2. **Background Rate-Limited Evacuation**:
   - A background migration worker iterates over all active object payloads residing on the draining drive.
   - For each object:
     1. Reads the payload from the source drive.
     2. Writes the payload to a healthy destination drive with adequate free space.
     3. Computes the SHA-256 checksum and compares it against the catalog record.
     4. Updates `objects.storage_path` in PostgreSQL atomically within a transaction.
     5. Deletes the old file from the source drive.
3. **State Transition to `DRAINED` / `SAFE_TO_REMOVE`**:
   - Once all objects have been migrated and verified, the drive status transitions to `DRAINED`.
   - The operator can safely unmount (`umount`) and physically detach the disk from the server.

### 5. Administrative Surface (Web Console & MCP)

- **Web Dashboard**: An enhanced Storage view displays each configured drive with real-time capacity progress bars, drive health indicators (`HEALTHY`, `WARNING`, `BLOCKED`, `DRAINING`), an "Add Drive" action, and an intuitive "Drain & Replace" workflow.
- **MCP Tools**:
  - `pontemesh_list_storage_drives`: Inspect all drives in the storage pool, their mount points, free bytes, and health statuses.
  - `pontemesh_add_storage_drive`: Register a new storage mount point dynamically into the pool.
  - `pontemesh_drain_storage_drive`: Trigger safe background evacuation of a drive prior to physical removal.

## Consequences

- **High Availability Storage**: Operators can add capacity on-the-fly and swap dying hardware drives without downtime or interrupted game/software downloads.
- **Simplicity**: No mandatory requirements for complex OS storage configurations (such as LVM volume groups or hardware RAID controllers); standard individual ext4/xfs/btrfs mount points can be attached directly.
- **Data Protection**: SHA-256 cryptographic verification during evacuation ensures that no file is corrupted or orphaned during disk replacement.
- **Zero Client Impact**: Client downloads and P2P transfers are completely unaffected by internal drive reorganization; all HTTP Range and fragment requests resolve smoothly through the updated catalog paths.
