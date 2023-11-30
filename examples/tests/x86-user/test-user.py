import cli
import simics

simics.SIM_load_module("tsffs")

tsffs = simics.SIM_create_object(simics.SIM_get_class("tsffs"), "tsffs", [])
simics.SIM_set_log_level(tsffs, 4)
tsffs.iface.tsffs.set_start_on_harness(True)
tsffs.iface.tsffs.set_stop_on_harness(True)
tsffs.iface.tsffs.set_timeout(3.0)
tsffs.iface.tsffs.set_generate_random_corpus(True)
tsffs.iface.tsffs.set_iterations(1000)
tsffs.iface.tsffs.set_use_snapshots(True)

simics.SIM_load_target(
    "qsp-x86/clear-linux",  # Target
    "qsp",  # Namespace
    [],  # Presets
    [["machine:hardware:storage:disk1:image", "test.fs.craff"]],
)

qsp = simics.SIM_get_object("qsp")

tsffs.iface.tsffs.add_architecture_hint(qsp.mb.cpu0.core[0][0], "i386")


# When we're running userspace code, we don't want to catch exeptions until
# we actually start fuzzing, including GPFs on other code. We can wait to
# enable the exception until later (we could even toggle it on and off per
# iteration)
def on_magic(o, e, r):
    # Wait for magic stop -- in reality this could wait for any stop
    # condition, but we make it easy on ourselves for testing purposes
    if r == 1:
        tsffs.iface.tsffs.add_exception_solution(13)


def startup_script_branch():
    cli.global_cmds.wait_for_global_time(seconds=20.0, _relative=True)
    qsp.serconsole.con.iface.con_input.input_str("sudo mkdir /disk0/\n")
    cli.global_cmds.wait_for_global_time(seconds=1.0, _relative=True)
    qsp.serconsole.con.iface.con_input.input_str("sudo mount /dev/sdb /disk0/\n")
    cli.global_cmds.wait_for_global_time(seconds=1.0, _relative=True)
    qsp.serconsole.con.iface.con_input.input_str("ls /disk0\n")
    cli.global_cmds.wait_for_global_time(seconds=1.0, _relative=True)
    qsp.serconsole.con.iface.con_input.input_str("/disk0/test\n")


def exit_script_branch():
    cli.global_cmds.wait_for_global_time(seconds=240.0, _relative=True)
    simics.SIM_quit(1)


simics.SIM_hap_add_callback("Core_Magic_Instruction", on_magic, None)
cli.sb_create(startup_script_branch)
cli.sb_create(exit_script_branch)

simics.SIM_continue(0)
# NOTE: If running from CLI, omit this!
simics.SIM_main_loop()
