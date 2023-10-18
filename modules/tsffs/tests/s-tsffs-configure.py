# Test that we can successfully load and configure the TSFFS module. 

import simics

simics.SIM_load_module("tsffs")
tsffs = SIM_create_object(SIM_get_class("tsffs"), "tsffs", [])
tsffs.iface.tsffs.set_start_on_harness(True)
tsffs.iface.tsffs.set_stop_on_harness(True)
tsffs.iface.tsffs.set_use_snapshots(True)
tsffs.iface.tsffs.set_timeout(60.0)
tsffs.iface.tsffs.add_exception_solution(6)
tsffs.iface.tsffs.add_exception_solution(14)
tsffs.iface.tsffs.remove_exception_solution(6)
tsffs.iface.tsffs.set_all_exceptions_are_solutions(True)
tsffs.iface.tsffs.set_all_exceptions_are_solutions(False)
tsffs.iface.tsffs.add_breakpoint_solution(0)
tsffs.iface.tsffs.add_breakpoint_solution(1)
tsffs.iface.tsffs.remove_breakpoint_solution(0)
tsffs.iface.tsffs.set_all_breakpoints_are_solutions(True)
tsffs.iface.tsffs.set_all_breakpoints_are_solutions(False)
tsffs.iface.tsffs.set_tracing_mode("once")
tsffs.iface.tsffs.set_cmplog_enabled(False)
tsffs.iface.tsffs.set_corpus_directory("%simics%/corpus/")
tsffs.iface.tsffs.set_solutions_directory("%simics%/solutions")
tsffs.iface.tsffs.set_generate_random_corpus(True)
tsffs_config = tsffs.iface.get_configuration()

