"""
Rules for building the printer
"""

load("//@star/packages/star/buildifier.star", "buildifier_add")
load("//@star/packages/star/rust.star", "rust_add")
load("//@star/packages/star/starship.star", "starship_add_bash")
load(
    "//@star/sdk/star/run.star",
    "run_add_exec",
    "run_add_exec_test",
)
load("//@star/sdk/star/spaces-env.star", "spaces_working_env")

rust_add(
    "rust_toolchain",
    version = "1.80",
)

buildifier_add(
    "buildifier",
    version = "v8.2.1",
)

starship_add_bash("starship0", shortcuts = {})
spaces_working_env(add_spaces_to_sysroot = True, inherit_terminal = False)

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
    args = ["fmt", "--check"],
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
