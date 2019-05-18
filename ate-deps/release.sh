#!/bin/bash -e
mvn clean
mvn release:prepare
mvn release:perform
