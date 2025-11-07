"""
Rules for building the printer
"""

load("//@star/packages/star/rust.star", "rust_add")
load("//@star/packages/star/buildifier.star", "buildifier_add")
load(
    "//@star/sdk/star/run.star",
    "run_add_exec",
    "run_add_exec_test",
)

rust_add(
    "rust_toolchain",
    version = "1.80",
)

buildifier_add(
    "buildifier",
    version = "v8.2.1",
)

run_add_exec(
    "check",
    command = "cargo",
    args = ["check"],
    help = "Run cargo check on workspace",
    working_directory = ".",
)

run_add_exec_test(
    "check_format",
    command = "cargo",
    args = ["--check", "fmt"],
    help = "Run cargo build on workspace",
    deps = ["check"],
    working_directory = ".",
)

run_add_exec(
    "build",
    command = "cargo",
    args = ["build"],
    help = "Run cargo build on workspace",
    working_directory = ".",
)

run_add_exec(
    "clippy",
    command = "cargo",
    args = ["clippy"],
    log_level = "Passthrough",
    help = "Run cargo clippy on workspace",
    working_directory = ".",
)
