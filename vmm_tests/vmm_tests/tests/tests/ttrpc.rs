// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Integration tests for hvlite's TTRPC interface.

#![cfg_attr(guest_arch = "aarch64", allow(unused_imports))]

use crate::prelude::*;
use anyhow::Context;
use guid::Guid;
use hvlite_ttrpc_vmservice as vmservice;
use pal_async::DefaultPool;
use petri_artifacts_vmm_test::artifacts;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::process::Stdio;
use unix_socket::UnixStream;
use vmm_test_petri_support::TestArtifactResolverExt;

#[cfg(guest_arch = "x86_64")]
#[test]
fn test_ttrpc_interface() -> anyhow::Result<()> {
    // This test doesn't use a Petri VM, so it needs to initialize tracing itself.
    test_with_tracing::init();

    let artifacts = vmm_tests_artifact_resolver()
        .require_hvlite_standard(None)
        .require(artifacts::loadable::LINUX_DIRECT_TEST_KERNEL_X64)
        .require(artifacts::loadable::LINUX_DIRECT_TEST_INITRD_X64)
        .finalize();

    let mut socket_path = std::env::temp_dir();
    socket_path.push(Guid::new_random().to_string());

    tracing::info!(socket_path = %socket_path.display(), "launching hvlite with ttrpc");

    let mut child = std::process::Command::new(artifacts.resolve(artifacts::OPENVMM_NATIVE))
        .arg("--ttrpc")
        .arg(&socket_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Wait for stdout to close.
    let mut stdout = child.stdout.take().context("failed to take stdout")?;
    let mut b = [0];
    assert_eq!(stdout.read(&mut b)?, 0);

    // Copy the child's stderr to this process's, since internally this is
    // wrapped by the test harness.
    let stderr = child.stderr.take().context("failed to take stderr")?;
    std::thread::spawn(move || {
        let stderr = BufReader::new(stderr);
        for line in stderr.lines() {
            tracing::info!(target: "stderr_log", "{}", line.unwrap());
        }
    });

    let kernel_path = artifacts.resolve(artifacts::loadable::LINUX_DIRECT_TEST_KERNEL_X64);
    let initrd_path = artifacts.resolve(artifacts::loadable::LINUX_DIRECT_TEST_INITRD_X64);

    let conn = UnixStream::connect(&socket_path)?;
    DefaultPool::run_with(|driver| async move {
        let client = mesh_rpc::Client::new(&driver, conn);
        for i in 0..3 {
            let mut com1_path = std::env::temp_dir();
            com1_path.push(Guid::new_random().to_string());

            client
                .call(
                    vmservice::Vm::CreateVm,
                    vmservice::CreateVmRequest {
                        config: Some(vmservice::VmConfig {
                            memory_config: Some(vmservice::MemoryConfig {
                                memory_mb: 256,
                                ..Default::default()
                            }),
                            processor_config: Some(vmservice::ProcessorConfig {
                                processor_count: 2,
                                ..Default::default()
                            }),
                            boot_config: Some(vmservice::vm_config::BootConfig::DirectBoot(
                                vmservice::DirectBoot {
                                    kernel_path: kernel_path.to_string_lossy().to_string(),
                                    initrd_path: initrd_path.to_string_lossy().to_string(),
                                    kernel_cmdline:
                                        "console=ttyS0 rdinit=/bin/busybox panic=-1 -- poweroff -f"
                                            .to_string(),
                                },
                            )),
                            serial_config: Some(vmservice::SerialConfig {
                                ports: vec![vmservice::serial_config::Config {
                                    port: 0,
                                    socket_path: com1_path.to_string_lossy().into(),
                                }],
                            }),
                            ..Default::default()
                        }),
                        log_id: String::new(),
                    },
                )
                .await
                .unwrap()
                .unwrap();

            let com1 = UnixStream::connect(&com1_path).unwrap();

            std::thread::spawn(move || {
                let read = BufReader::new(com1);
                for line in read.lines() {
                    match line {
                        Ok(line) => tracing::info!(target: "linux_console", "{}", line),
                        Err(e) => tracing::error!(target: "linux_console", "{}", e),
                    }
                }
            });

            assert_eq!(
                client
                    .start_call(
                        vmservice::Vm::WaitVm,
                        (),
                        Some(std::time::Duration::from_millis(100)),
                    )
                    .await
                    .unwrap()
                    .unwrap_err()
                    .code,
                mesh_rpc::service::Code::DeadlineExceeded as i32
            );

            let waiter = client.start_call(vmservice::Vm::WaitVm, (), None);

            match i {
                0 | 2 => {
                    client
                        .call(vmservice::Vm::ResumeVm, ())
                        .await
                        .unwrap()
                        .unwrap();

                    waiter.await.unwrap().unwrap();

                    if i == 0 {
                        client
                            .call(vmservice::Vm::TeardownVm, ())
                            .await
                            .unwrap()
                            .unwrap();

                        client
                            .call(vmservice::Vm::WaitVm, ())
                            .await
                            .unwrap()
                            .unwrap_err();
                    } else {
                        let _ = client.call(vmservice::Vm::Quit, ()).await;
                    }
                }
                1 => {
                    client
                        .call(vmservice::Vm::TeardownVm, ())
                        .await
                        .unwrap()
                        .unwrap();

                    waiter.await.unwrap().unwrap_err();
                }
                _ => unreachable!(),
            }
        }
    });

    child.wait()?;
    let _ = std::fs::remove_file(&socket_path);

    Ok(())
}
