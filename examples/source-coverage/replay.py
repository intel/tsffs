# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

"""
Replay saved TSFFS inputs one by one in a Simics session.

Load this file from a Simics script with `run-script`, after `init-tsffs`:

    run-script "%simics%/replay.py"
    @tsffs_replay_arm("%simics%/corpus")
    # load the target and boot it up to the harness
    run
    @tsffs_replay("%simics%/corpus")

`tsffs_replay_arm` is called before the target runs. The first `run` then ends with
the simulation stopped at the harness, with the TSFFS snapshot taken. `tsffs_replay`
runs every input from that snapshot and prints the stop reason of each.
"""

import os

import cli
import conf
import simics


def tsffs_replay_inputs(directory):
    """
    Return the absolute paths of the inputs directly inside `directory`, sorted by name.

    Only regular files are inputs. Hidden files (the lock and metadata files LibAFL
    keeps next to each corpus entry) and subdirectories are skipped. `directory` may
    start with `%simics%`.

    Raises `ValueError` when the directory is missing or holds no input.
    """
    resolved = simics.SIM_lookup_file(directory)

    if resolved is None or not os.path.isdir(resolved):
        raise ValueError(f"no inputs to replay in {directory}")

    inputs = sorted(
        os.path.abspath(entry.path)
        for entry in os.scandir(resolved)
        if entry.is_file() and not entry.name.startswith(".")
    )

    if not inputs:
        raise ValueError(f"no inputs to replay in {directory}")

    return inputs


def tsffs_replay_arm(directory):
    """
    Prepare TSFFS to stop at the harness with the first input of `directory` written.

    Call this before the target runs. The next `run` ends with the simulation stopped at
    the harness start, ready for `tsffs_replay`.
    """
    inputs = tsffs_replay_inputs(directory)

    conf.tsffs.repro_auto_continue = False
    conf.tsffs.iface.fuzz.repro(inputs[0])


def tsffs_replay(directory):
    """
    Run every input of `directory` from the TSFFS snapshot, one after the other.

    Prints one `TSFFS_REPLAY <i>/<n> <name> <stop reason>` line per input, then
    `TSFFS_REPLAY done <n>`.

    Sets `repro_auto_continue` to `False` and leaves it so: this loop resumes the
    simulation itself, once per input.
    """
    inputs = tsffs_replay_inputs(directory)
    total = len(inputs)

    # Without this, `repro` also queues a resume of its own and an input can run past
    # its stop. `tsffs_replay_arm` sets it too, but this function may be called on a
    # session that was not armed, for example after fuzzing.
    conf.tsffs.repro_auto_continue = False

    for index, path in enumerate(inputs, start=1):
        conf.tsffs.iface.fuzz.repro(path)
        cli.quiet_run_command("run")
        print(
            f"TSFFS_REPLAY {index}/{total} {os.path.basename(path)} "
            f"{conf.tsffs.last_stop_reason}"
        )

    print(f"TSFFS_REPLAY done {total}")
