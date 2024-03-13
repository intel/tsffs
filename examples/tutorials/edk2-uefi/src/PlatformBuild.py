# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

"""
Script to build Tutorial firmware

See: https://www.tianocore.org/edk2-pytool-extensions/integrate/porting/
for more info
"""

from os.path import abspath, dirname, join
from typing import Iterable, List

from edk2toolext.environment.uefi_build import UefiBuilder
from edk2toolext.invocables.edk2_platform_build import BuildSettingsManager
from edk2toolext.invocables.edk2_setup import RequiredSubmodule, SetupSettingsManager
from edk2toolext.invocables.edk2_update import UpdateSettingsManager


class TutorialSettingsManager(
    UpdateSettingsManager, SetupSettingsManager, BuildSettingsManager
):
    """
    Settings manager for TutorialEFI application build
    """

    def __init__(self) -> None:
        """
        Initialize the settings manager
        """
        script_path = dirname(abspath(__file__))

        # Initialize the workspace (ws) path
        self.ws = script_path

    def GetWorkspaceRoot(self) -> str:
        """
        Returns the absolute path to the workspace root
        """
        return self.ws

    def GetActiveScopes(self) -> List[str]:
        """
        Returns scope names this settings manager will remain active for
        """
        return ["Tutorial"]

    def GetPackagesSupported(self) -> Iterable[str]:
        """
        Returns paths from the edk2 repository root of edk2 packages
        supported by this build
        """
        return ("Tutorial",)

    def GetRequiredSubmodules(self) -> Iterable[RequiredSubmodule]:
        """
        Returns submodules required for this package.
        """
        # We don't have any required submodules, so we just return an empty list.
        return []

    def GetArchitecturesSupported(self) -> Iterable[str]:
        """
        Returns edk2 architectures supported by this build.
        """
        return ("X64",)

    def GetTargetsSupported(self) -> Iterable[str]:
        """
        Returns target tags supported by this build.
        """
        return ("DEBUG",)

    def GetPackagesPath(self) -> Iterable[str]:
        """
        Returns the paths to the edk2 package
        """
        return [abspath(join(self.GetWorkspaceRoot(), ".."))]


class PlatformBuilder(UefiBuilder):
    """
    Platform build for Tutorial module
    """

    def SetPlatformEnv(self) -> int:
        """
        Set environment variables for the platform
        """
        self.env.SetValue(
            "ACTIVE_PLATFORM", "Tutorial/Tutorial.dsc", "Platform hardcoded"
        )
        self.env.SetValue("PRODUCT_NAME", "Tutorial", "Platform hardcoded")
        self.env.SetValue("TARGET_ARCH", "X64", "Platform hardcoded")
        self.env.SetValue("TOOL_CHAIN_TAG", "GCC", "Platform Hardcoded", True)

        return 0
