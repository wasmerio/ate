#!/bin/bash -e
rm -f target/*.jar || true
rm -r -f bin-lib || true
[ -e repo.zip ] && rm -f repo.zip || true
[ -e target/libs.zip ] && rm -f target/libs.zip || true

gradle build
