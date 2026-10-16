# TSFFS

Snapshot-based fuzzer for firmware and software running inside Simics, built on LibAFL.

## Language

**Edge**:
An occupied slot in the fuzzer's coverage map, as tracked by the map feedback's history map. This is the AFL meaning: `edges_found` and `bitmap_cvg` count Edges.
_Avoid_: using "edge" for a branch target address

**Branch target**:
A distinct program counter reached by a taken branch, recorded by the tracer when coverage reporting is enabled. Reported in `fuzzer_stats` as `tsffs_branch_targets`, never as `edges_found`.
_Avoid_: edge

**Instance**:
One fuzzing session's AFL-compatible output directory, named by `instance_name` (default `default`) under the output directory. At most one live session holds an Instance at a time; parallel sessions need distinct names.
_Avoid_: run, job

## Relationships

- Several **Branch targets** can hash into the same **Edge**, and one **Branch target** can contribute to several **Edges**; the two counts are not comparable.

## Flagged ambiguities

- The tracer's `edges_seen` field holds **Branch targets**, not **Edges**; the name predates this glossary.
