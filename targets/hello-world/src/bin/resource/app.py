from sim_params import params
import simics
import commands
import io, contextlib

args = [
    [name, commands.param_val_to_str(value)] for (name, value) in params.items()
]

simics.SIM_run_command_file_params(
    simics.SIM_lookup_file("%simics%/targets/hello-world/run-uefi-app.simics"),
    True, args
)

if SIM_get_batch_mode():
    SIM_log_info(
        1,
        conf.sim,
        0,
        'Batch mode detected. Disconnecting console from VGA'
    )
    conf.board.mb.gpu.vga.console=None


SIM_load_module("confuse_module")
SIM_create_object('confuse_module', 'confuse_module', [])
conf.confuse_module.iface.confuse_module.add_processor(SIM_get_object(simenv.system).mb.cpu0.core[0][0])
conf.confuse_module.iface.confuse_module.start()
print("Started simulation")
SIM_main_loop()