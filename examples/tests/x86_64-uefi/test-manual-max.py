import cli
import simics

simics.SIM_load_module("tsffs")

tsffs = simics.SIM_create_object(simics.SIM_get_class("tsffs"), "tsffs", [])
simics.SIM_set_log_level(tsffs, 4)
tsffs.iface.tsffs.set_start_on_harness(False)
tsffs.iface.tsffs.set_stop_on_harness(False)
tsffs.iface.tsffs.set_timeout(3.0)
tsffs.iface.tsffs.add_exception_solution(14)
tsffs.iface.tsffs.set_generate_random_corpus(True)
tsffs.iface.tsffs.set_iterations(1000)
tsffs.iface.tsffs.set_use_snapshots(False)

simics.SIM_load_target(
    "qsp-x86/uefi-shell",  # Target
    "qsp",  # Namespace
    [],  # Presets
    [  # Cmdline args
        ["machine:hardware:storage:disk0:image", "minimal_boot_disk.craff"],
        ["machine:hardware:processor:class", "x86-goldencove-server"],
    ],
)

qsp = simics.SIM_get_object("qsp")


def on_magic(o, e, r):
    # Wait for magic stop -- in reality this could wait for any stop
    # condition, but we make it easy on ourselves for testing purposes
    if r == 2:
        print("Got magic stop...")
        tsffs.iface.tsffs.stop()


def start_script_branch():
    # Wait for magic start -- in reality this could wait for any
    # start condition, but we make it easy on ourselves for testing purposes
    print("Waiting for magic start...")
    conf.bp.magic.cli_cmds.wait_for(number=1)
    print("Got magic start...")

    # In reality, you probably have a known buffer in mind to fuzz
    testcase_address_regno = conf.qsp.mb.cpu0.core[0][0].iface.int_register.get_number(
        "rdi"
    )
    print("testcase address regno: ", testcase_address_regno)
    testcase_address = conf.qsp.mb.cpu0.core[0][0].iface.int_register.read(
        testcase_address_regno
    )
    print("testcase address: ", testcase_address)
    maximum_size = 8
    virt = False

    print(
        "Starting with testcase address",
        hex(testcase_address),
        "maximum size",
        hex(maximum_size),
        "virt",
        virt,
    )

    tsffs.iface.tsffs.start_with_maximum_size(
        conf.qsp.mb.cpu0.core[0][0],
        testcase_address,
        maximum_size,
        True,
    )


def startup_script_branch():
    cli.global_cmds.wait_for_global_time(seconds=15.0, _relative=True)
    qsp.serconsole.con.iface.con_input.input_str("\n")
    cli.global_cmds.wait_for_global_time(seconds=1.0, _relative=True)
    qsp.serconsole.con.iface.con_input.input_str("FS0:\n")
    cli.global_cmds.wait_for_global_time(seconds=1.0, _relative=True)
    cli.global_cmds.start_agent_manager()
    qsp.serconsole.con.iface.con_input.input_str(
        "SimicsAgent.efi --download "
        + simics.SIM_lookup_file("%simics%/test.efi")
        + "\n"
    )
    cli.global_cmds.wait_for_global_time(seconds=3.0, _relative=True)
    qsp.serconsole.con.iface.con_input.input_str("test.efi\n")


def exit_script_branch():
    cli.global_cmds.wait_for_global_time(seconds=240.0, _relative=True)
    simics.SIM_quit(1)


simics.SIM_hap_add_callback("Core_Magic_Instruction", on_magic, None)
cli.sb_create(start_script_branch)
cli.sb_create(startup_script_branch)
cli.sb_create(exit_script_branch)

simics.SIM_continue(0)
# NOTE: If running from CLI, omit this!
simics.SIM_main_loop()
