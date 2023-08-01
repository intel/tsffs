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

conf.tsffs_module.iface.tsffs_module.init()
conf.tsffs_module.iface.tsffs_module.add_processor(
    SIM_get_object(simenv.system).mb.cpu0.core[0][0]
)
conf.tsffs_module.iface.tsffs_module.add_fault(14)
conf.tsffs_module.iface.tsffs_module.add_fault(6)
SIM_log_info(1, conf.sim, 0, "Added processor")
SIM_log_info(1, conf.sim, 0, "Started module")

SIM_log_info(1, conf.sim, 0, "Started simulation")
