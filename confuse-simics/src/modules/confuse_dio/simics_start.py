# © 2014 Intel Corporation
#
# This software and the related documents are Intel copyrighted materials, and
# your use of them is governed by the express license under which they were
# provided to you ("License"). Unless the License provides otherwise, you may
# not use, modify, copy, publish, distribute, disclose or transmit this software
# or the related documents without Intel's prior written permission.
#
# This software and the related documents are provided as is, with no express or
# implied warranties, other than those that are expressly stated in the License.


import cli
import simics

def get_dummy_uut_remote_control():
    all_objs = simics.VT_get_all_instances("dummy_uut_remote_control")
    if not all_objs:
        return None
    return all_objs[0]  # There can be only one

def ok_msg(name, created):
    if not created:
        return "dummy_uut_remote_control '%s' is already started." % name
    return "'%s' is created and enabled." % name

def new_dummy_uut_remote_control_cmd(name):
    created = False
    dummy_urc = get_dummy_uut_remote_control()
    if not dummy_urc:
        try:
            dummy_urc = simics.SIM_create_object("dummy_uut_remote_control", name, [])
        except simics.SimExc_General as e:
            raise cli.CliError(str(e))
        created = True
    elif dummy_urc.name != name:
        raise cli.CliError("A dummy_uut_remote_control already exists as '%s'."
                       % bridge_t2h.name)
    return cli.command_return(value=dummy_urc,
                          message=ok_msg(name, created))

cli.new_command("start-dummy-uut-remote-control", new_dummy_uut_remote_control_cmd,
            [cli.arg(cli.str_t, "name", "?", "i_dummy_uut_remote_control")],
            type = ["magic_pipe", "dummy_uut_remote_control"],
            short = "create and enable the Magic pipe dummy UUT remote control",
            doc = """
    Create and enable the dummy UUT remote control, which is an example of using
    the magic pipe library. In this simple example, only one such control can exist.
    in the simulation.

    The <arg>name</arg> argument is optional and defaults to
    "i_dummy_uut_remote_control".""")

#register our notifier
import simics
ntfy_id = simics.SIM_notifier_type('MAGICPIPE_from_harness')
simics.SIM_register_notifier('dummy_uut_remote_control', ntfy_id, "Triggers when magic pipe has delivered data and needs a response.")