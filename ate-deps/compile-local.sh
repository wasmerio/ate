#!/bin/bash -e

if [[ -z "$1" ]]; then
  echo "You must provide the version number" 1>&2
  exit 1
fi
VERSION=$1

mvn clean
mvn compile package
mvn install:install-file -Dfile=pom.xml -DgroupId=com.tokera -DartifactId=ate-deps -Dversion=$VERSION -Dpackaging=pom -DpomFile=pom.xml
mvn install:install-file -Dfile=pom.xml -DgroupId=com.tokera -DartifactId=ate-deps -Dversion=$VERSION -Dpackaging=pom -DpomFile=pom.xml -DlocalRepositoryPath=../../tokera/tokapi/.m2
