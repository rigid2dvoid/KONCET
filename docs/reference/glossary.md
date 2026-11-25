# Glossary

> **Status:** Baseline · **Last updated:** 2026-11-24

Terms are listed as used in product copy, documentation, and code. Code identifiers should use
the English term; UI copy may phrase it naturally but must not imply a real effect.

| Term | Definition |
| --- | --- |
| **Active run** | The single run currently accepting simulation commands. Only one is allowed in `v0.1`. |
| **App process** | The simulated process supervised inside a simulated container. It never executes. |
| **Build stage** | One of Validate, Prepare, Build, Package. Fixed order, deterministic duration. |
| **Command trace** | Ordered record of user commands with simulated timestamps, used for reproducibility. |
| **Container engine (simulated)** | Model that creates and manages simulated containers from simulated images. Not Docker/Podman. |
| **Deployment** | The simulated service exposed by a run, with derived status/health and an endpoint. |
| **Determinism** | Same engine version + inputs + command trace ⇒ byte-equivalent observable output. |
| **Digest** | Simulated content-addressed SHA-256 identifier for an image manifest. Not a registry digest. |
| **Engine version** | Version of the simulation semantics, recorded on each run. Equals the product version in `v0.1`. |
| **Event** | Immutable, ordered domain fact persisted before UI publication. |
| **Firewall (simulated)** | Ordered policy rules evaluated for simulated traffic. Does not touch host firewall. |
| **Golden run** | Test fixture comparing canonical simulator output for fixed inputs. |
| **Image (simulated)** | Immutable local artifact with layers, tag, digest, and size; not an OCI image. |
| **Preset** | Built-in scenario used as a starting point (Happy Path, Build Failure, Blocked Port, Crash Loop). |
| **Primary reason** | Stable code + human explanation for a non-healthy deployment status. |
| **Project** | Authoring unit owning virtual source files, scenarios, and run history. |
| **Proxy (simulated)** | Virtual host/path route to a service port; performs simulated connectivity probes. |
| **Run** | One deterministic simulation session bound to snapshots, seed, versions, and command trace. |
| **Scenario** | Versioned JSON configuration for build, service, networking, faults, and limits. |
| **Simulation time** | Virtual elapsed time in milliseconds; advanced by fixed 100 ms ticks. |
| **Source snapshot** | Immutable copy of the virtual source tree captured at run start. |
| **Source tree** | Virtual files stored in SQLite; never mapped to host files or executed. |
| **Virtual time** | See simulation time. |
| **Xvfb** | Virtual framebuffer used to run Linux UI tests headlessly. |
| **`.koncet.local`** | Reserved simulated hostname suffix; not resolved on the network. |
