#!/bin/bash -e

mvn clean
mvn compile
mvn package
mvn release:prepare
mvn release:perform
