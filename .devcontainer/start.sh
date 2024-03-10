#!/usr/bin/env bash

set -e

rm -rf /workspace/.bin
ln -s /tmp/build-cache/.bin /workspace/.bin
ls /workspace/ | grep target | grep -q 'oatmeal' || sudo chown -R oatmeal:oatmeal /workspace/target
