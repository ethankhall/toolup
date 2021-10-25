#!/usr/bin/env bash

set -eu

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

rm -rf $SCRIPT_DIR/../tmp || true
mkdir $SCRIPT_DIR/../tmp

cargo build
COMMAND_PATH=$SCRIPT_DIR/../target/debug/toolup
SHIM_PATH=$SCRIPT_DIR/../target/debug/toolup-shim
export TOOLUP_ROOT_TOOL_DIR=$SCRIPT_DIR/../tmp/tools

# Package V1 of the test binary
$COMMAND_PATH package archive \
    --config $SCRIPT_DIR/../examples/package/hello-world-v1/package.toml \
    --target-dir $SCRIPT_DIR/../examples/package/hello-world-v1 \
    --archive-dir $SCRIPT_DIR/../tmp

# Package V2 of the test binary
$COMMAND_PATH package archive \
    --config $SCRIPT_DIR/../examples/package/hello-world-v2/package.toml \
    --target-dir $SCRIPT_DIR/../examples/package/hello-world-v2 \
    --archive-dir $SCRIPT_DIR/../tmp

export TOOLUP_GLOBAL_CONFIG_DIR=$SCRIPT_DIR/../tmp/config

$COMMAND_PATH package install \
    --archive-path $SCRIPT_DIR/../tmp/hello_world-1.0.0.tar.gz \
    --overwrite

$COMMAND_PATH package install \
    --archive-path $SCRIPT_DIR/../tmp/hello_world-1.0.1.tar.gz \
    --overwrite

$COMMAND_PATH exec hello-world
OUTPUT="$($COMMAND_PATH exec hello-world)"
if [ "$OUTPUT" != "Goodbye World!" ]; then
    echo "Current version did not return with expected output"
    exit 1
fi

$COMMAND_PATH exec --version 1.0.0 hello-world
OUTPUT="$($COMMAND_PATH exec --version 1.0.0 hello-world)"
if [ "$OUTPUT" != "hello world!" ]; then
    echo "Version 1.0.0 did not return with expected output"
    exit 1
fi

ln -s $SHIM_PATH $SCRIPT_DIR/../tmp/hello-world

$SCRIPT_DIR/../tmp/hello-world
OUTPUT="$($SCRIPT_DIR/../tmp/hello-world)"
if [ "$OUTPUT" != "Goodbye World!" ]; then
    echo "Current version did not return with expected output"
    exit 1
fi