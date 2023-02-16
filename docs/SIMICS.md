# Simics Setup

When building this software, you can provide a global SIMICS install to speed up the
build process by setting the environment variable `SIMICS_HOME`.

## Install Simics GUI Dependencies

```sh
$ sudo apt-get install libatk1.0-0 libatk-bridge2.0-0 libcups2 libgtk-3-0 libgbm1 \
    libasound2
```

## Download Simics

You can download Simics from the link on the
[public release page](https://www.intel.com/content/www/us/en/developer/articles/tool/simics-simulator.html).

You will need to download both files:

- `intel-simics-package-manager-1.5.3-linux64.tar.gz`
- `simics-6-packages-2022-49-linux64.ispm`

In this case, we'll assume you have downloaded both files to the `~/Downloads`
directory, which you can do by running:

```sh
$ mkdir -p ~/Downloads
$ wget https://registrationcenter-download.intel.com/akdlm/IRC_NAS/708028d9-b710-45ea-baab-3b9c78c32cfc/intel-simics-package-manager-1.5.3-linux64.tar.gz \
    -O ~/Downloads/intel-simics-package-manager-1.5.3-linux64.tar.gz
$ wget https://registrationcenter-download.intel.com/akdlm/IRC_NAS/708028d9-b710-45ea-baab-3b9c78c32cfc/simics-6-packages-2022-49-linux64.ispm \
    -O ~/Downloads/simics-6-packages-2022-49-linux64.ipsm
```

## Install Simics

Assuming the two download locations above, we will install Simics to `~/install/simics`.

```sh
$ mkdir -p ~/install/simics/
$ tar -C ~/install/simics -xzvf ~/Downloads/intel-simics-package-manager-1.5.3-linux64.tar.gz
$ ~/install/simics/intel-simics-package-manager-1.5.3/ispm packages \
    --install-dir ~/install/simics \
    --install-bundle ~/Downloads/simics-6-packages-2022-49-linux64.ispm \
    --non-interactive
```

