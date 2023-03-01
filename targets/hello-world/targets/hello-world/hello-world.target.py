# One day we'll learn how to call back into our rust code from a stub app script, thus completing
# the carcinization of simics programming. Until then:
from sim_params import params
import simics
import commands
import io
import contextlib

args = list(map(lambda p: [p[0], commands.param_val_to_str(p[1])], params.items()))

run_uefi_app_simics_script = simics.SIM_lookup_file(
    "%simics%/targets/hello-world/run.simics"
)

simics.SIM_run_command_file_params(
    run_uefi_app_simics_script,
    True,
    args,
)

if SIM_get_batch_mode():
    SIM_log_info(1, conf.sim, 0, "Batch mode detected. Disconnecting console from VGA")
    conf.board.mb.gpu.vga.console = None

# Reach start state of test (indicated by MAGIC(42) in on-target test harness
SIM_run_command("bp.hap.run-until name = Core_Magic_Instruction index = 42")

# Critical piece, this loads our module
SIM_create_object("minimal_simics_module", "msm", [])

# Enable in memory snapshot feature
SIM_run_command("enable-unsupported-feature internals")
# SIM_run_command('enable-unsupported-feature selfprof')

SIM_run_command("save-snapshot name = origin")

# Check that we have our snapshot as index 0 (which is currently hard coded in the restore code
cmd_output = io.StringIO()

with contextlib.redirect_stdout(cmd_output):
    SIM_run_command("list-snapshots")

snapshot_list = cmd_output.getvalue()

ckpt_id = int(
    next(
        filter(
            lambda l: len(l) > 2 and l[1].strip() == "origin",
            map(lambda l: l.split(), snapshot_list.splitlines()),
        )
    )[0]
)

if ckpt_id != 0:
    SIM_log_error(conf.fuzz_if, 0, "Microcheckpoint ID %d. Must be zero!" % (ckpt_id))
else:
    SIM_log_info(1, conf.fuzz_if, 0, "Microcheckpoint ID %d" % (ckpt_id))
