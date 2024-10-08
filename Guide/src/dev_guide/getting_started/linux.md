# Getting started on Linux / WSL2

This page provides instructions for installing the necessary dependencies to
build OpenVMM or OpenHCL on Linux / WSL2.

## \[WSL2] Installing WSL2

To install Windows Subsystem for Linux, run the following command in an
elevated Powershell window:

```powershell
PS> wsl --install
```

This should install WSL2 using the default Ubuntu linux distribution.
You can check that the installation completed successfully by running the
following command in a Powershell window.
```powershell
PS> wsl -l -v
  NAME            STATE           VERSION
* Ubuntu          Running         2
```
Once that command has completed, you will need to open WSL to complete the
installation and set your password. You can open WSL by typing `wsl` or `bash`
into Command Prompt or Powershell, or by opening the "Ubuntu" Windows Terminal
profile that should have been created.

> If you intend to cross-compile OpenVMM for Windows, please ensure you are
> running a recent version of Windows 11. Windows 10 is no longer supported as a
> development platform, due to needed WHP APIs.

All subsequent commands on this page must be run within WSL2.

## Installing Rust

To build OpenVMM or OpenHCL, you first need to install Rust.

The OpenVMM project actively tracks the latest stable release of Rust, though it
may take a week or two after a new stable is released until OpenVMM switches
over to it.

Please follow the [official instructions](https://www.rust-lang.org/tools/install) to do so.

## Cloning the OpenVMM source

**If using WSL2:** Do NOT clone the repo into Windows then try to access said
clone from Linux. It will result in serious performance issues.

```bash
$ cd path/to/where/you/clone/repos
$ git clone https://github.com/microsoft/openvmm.git
```

## Next Steps

You are now ready to build [OpenVMM](./build_openvmm.md) or
[OpenHCL](./build_openhcl.md)!