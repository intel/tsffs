# TSFFS

Snapshot-based fuzzer for firmware and software running inside Simics, built on LibAFL.

## Language

**Edge**:
An occupied slot in the fuzzer's coverage map, as tracked by the map feedback's history map. This is the AFL meaning: `edges_found` and `bitmap_cvg` count Edges.
_Avoid_: using "edge" for a branch target address

**Branch target**:
A distinct program counter reached by a taken branch, recorded by the tracer when coverage reporting is enabled. Reported in `log.json` heartbeats, never in AFL stat files.
_Avoid_: edge

**Instance**:
One fuzzing session's AFL-compatible stats view, a directory named by `instance_name` under the output directory. Unique per session unless the user names it.
_Avoid_: run, job

## Relationships

- Several **Branch targets** can hash into the same **Edge**, and one **Branch target** can contribute to several **Edges**; the two counts are not comparable.

## Flagged ambiguities

- The tracer's `edges_seen` field holds **Branch targets**, not **Edges**; the name predates this glossary.
