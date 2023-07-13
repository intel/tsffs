"""
App startup script for hello-world example
"""

# mypy: ignore-errors
# flake8: noqa
# pylint: disable=undefined-variable,import-error


import commands
import simics
from sim_params import params

SIM_log_info(1, conf.sim, 0, "Running app.py")

args = [[name, commands.param_val_to_str(value)] for (name, value) in params.items()]

SIM_log_info(1, conf.sim, 0, "Running with args" + str(args))

SIM_log_info(1, conf.sim, 0, "Running simics script")

simics.SIM_run_command_file_params(
    simics.SIM_lookup_file("%simics%/targets/hello-world/run-uefi-app.simics"),
    True,
    args,
)

SIM_log_info(1, conf.sim, 0, "Ran simics script")

if SIM_get_batch_mode():
    SIM_log_info(1, conf.sim, 0, "Got batch mode")
    SIM_log_info(1, conf.sim, 0, "Batch mode detected. Disconnecting console from VGA")
    conf.board.mb.gpu.vga.console = None

SIM_log_info(1, conf.sim, 0, "Done disconnecting")


SIM_load_module("tsffs_module")
SIM_log_info(1, conf.sim, 0, "Loaded module")
try:
    SIM_create_object("tsffs_module", "tsffs_module", [])
    SIM_log_info(1, conf.sim, 0, "Created object")
except simics.SimExc_General as e:
    # SIM_get_object("tsffs_module", "tsffs_module")
    SIM_log_info(1, conf.sim, 0, "Module object already exists: " + str(e))

conf.tsffs_module.iface.tsffs_module.add_processor(
    SIM_get_object(simenv.system).mb.cpu0.core[0][0]
)
SIM_log_info(1, conf.sim, 0, "Added processor")
conf.tsffs_module.iface.tsffs_module.start(True)
SIM_log_info(1, conf.sim, 0, "Started module")

SIM_log_info(1, conf.sim, 0, "Started simulation")
SIM_main_loop()
