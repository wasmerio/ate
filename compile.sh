#!/bin/bash -e
[ -e target/ate-0.1.jar ] && rm -f target/ate-0.1.jar || true
[ -e repo.zip ] && rm -f repo.zip || true
[ -e target/libs.zip ] && rm -f target/libs.zip || true

mkdir -p /maven
export MAVEN_OPTS="-Dmaven.repo.local=/maven"

mvn -T 2C compile test package

echo "Hash: $(cat target/ate-0.1.jar | md5sum)"
cd target/lib; zip -r ../libs.zip *
cd ../..

mkdir -p bin-lib
mvn install:install-file -Dfile=target/ate-0.1.jar -DgroupId=com.tokera -DartifactId=ate -Dversion=0.1 -Dpackaging=jar -DlocalRepositoryPath=bin-lib -DpomFile=pom.xml
zip -r repo.zip bin-lib
