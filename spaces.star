"""
Rules for building the printer
"""

load(
    "//@star/sdk/star/run.star",
    "run_add_exec",
    "run_add_exec_test",
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
    deps = "check",
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
