# Test that we can successfully load the TSFFS module and create the TSFFS object

import simics

simics.SIM_load_module("tsffs")
tsffs = SIM_create_object(SIM_get_class("tsffs"), "tsffs", [])
