#!/bin/bash

set -ex

mkdir -p target
cd target
curl -O https://repo.maven.apache.org/maven2/com/fermyon/teavm-cli/0.2.8/teavm-cli-0.2.8.jar
curl -O https://repo.maven.apache.org/maven2/com/fermyon/teavm-interop/0.2.8/teavm-interop-0.2.8.jar
