#!/usr/bin/env bash

set -e

sudo chown -R oatmeal:oatmeal /workspace/.bin
cp -fr /tmp/build-cache/.bin /workspace/
ls /workspace/ | grep target | grep -q 'oatmeal' || sudo chown -R oatmeal:oatmeal /workspace/target
