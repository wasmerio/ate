#!/bin/bash -e
export MAVEN_OPTS="-Dmaven.repo.local=/maven"
mvn -T 2C package
