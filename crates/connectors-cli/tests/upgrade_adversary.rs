//! Compatibility and dispatch probes for the compiled upgrade report.

use std::future::Future;
use std::process::{Command, ExitCode, Stdio};
use std::task::{Context, Poll, Waker};

#[test]
fn embedding_upgrade_completes_on_first_poll_without_a_tokio_runtime() {
    let mut future = std::pin::pin!(connectors_cli::run_from([
        "embedded-connectors",
        "inspect",
        "upgrade",
        "-o",
        "json",
    ]));
    let mut context = Context::from_waker(Waker::noop());
    assert_eq!(
        future.as_mut().poll(&mut context),
        Poll::Ready(ExitCode::SUCCESS)
    );
}

#[test]
fn synchronous_dispatch_does_not_claim_other_commands_or_parser_errors() {
    for arguments in [
        vec!["connectors", "inspect", "providers"],
        vec!["connectors", "inspect", "upgrade", "--unknown"],
        vec!["connectors", "inspect", "upgrade", "--help"],
        vec!["connectors", "--version"],
    ] {
        assert_eq!(
            connectors_cli::run_without_runtime_from(arguments.clone()),
            None,
            "fast path consumed {arguments:?}"
        );
    }
}

#[test]
fn output_flags_at_each_clap_depth_keep_upgrade_synchronous_and_equivalent() {
    let mut expected = None;
    for format in ["text", "compact", "json", "yaml"] {
        let mut first_rendering = None;
        for arguments in [
            vec!["--output", format, "inspect", "upgrade"],
            vec!["inspect", "-o", format, "upgrade"],
            vec!["inspect", "upgrade", "--output", format],
        ] {
            let output = Command::new(env!("CARGO_BIN_EXE_connectors"))
                .args(&arguments)
                .env("TOKIO_WORKER_THREADS", "0")
                .stdin(Stdio::null())
                .output()
                .unwrap();
            assert!(output.status.success(), "{arguments:?}: {output:?}");
            assert!(output.stderr.is_empty(), "{output:?}");
            if let Some(first) = &first_rendering {
                assert_eq!(&output.stdout, first, "{arguments:?}");
            } else {
                first_rendering = Some(output.stdout.clone());
            }
            if format == "json" {
                expected =
                    Some(serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap());
            } else if format == "yaml" {
                let actual: serde_json::Value = serde_norway::from_slice(&output.stdout).unwrap();
                assert_eq!(Some(actual), expected);
            }
        }
    }
}
