# © 2010 Intel Corporation

import simics

# Extend this function if your device requires any additional attributes to be
# set. It is often sensible to make additional arguments to this function
# optional, and let the function create mock objects if needed.
def create_confuse_module(name = None):
    '''Create a new confuse_module object'''
    confuse_module = simics.pre_conf_object(name, 'confuse_module')
    simics.SIM_add_configuration([confuse_module], None)
    return simics.SIM_get_object(confuse_module.name)
