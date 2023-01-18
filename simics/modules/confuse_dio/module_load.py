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

def get_info(obj):
    return [(None, [])]

def get_status(obj):
    rows = [("Pipe", obj.pipe.name if obj.pipe else "None")]
    rows += [("Haps", obj.haps)]
    rows += [("Msg from target", obj.from_target)]
    rows += [("Msg to target", obj.to_target  if obj.to_target else "")]
    rows += [("Magic number", hex(obj.magic))]
    return [(None, rows)]

cli.new_info_command('dummy_uut_remote_control', get_info)
cli.new_status_command('dummy_uut_remote_control', get_status)


