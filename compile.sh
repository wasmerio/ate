#!/bin/bash -e
[ -e target/ate-0.1.jar ] && rm -f target/ate-0.1.jar || true

mkdir -p /maven
export MAVEN_OPTS="-Dmaven.repo.local=/maven"

mvn -T 2C compile test package

echo "Hash: $(cat target/ate-0.1.jar | md5sum)"
cd target/lib; zip -r ../libs.zip *
