#!/bin/bash -e
rm -f target/*.jar || true
[ -e repo.zip ] && rm -f repo.zip || true
[ -e target/libs.zip ] && rm -f target/libs.zip || true

mkdir -p /maven
export MAVEN_OPTS="-Dmaven.repo.local=/maven"

mvn -T 2C compile test package

JAR=$(basename target/*.jar)
[ -z "$JAR" ] && echo "No JAR file exists" && exit 1
VERSION=$(echo $JAR | cut -d "-" -f2 | sed 's|\.jar||g')
[ -z "$VERSION" ] && echo "Version could not be determined" && exit 1

echo "Hash: $(cat target/$JAR | md5sum)"
cd target/lib; zip -r ../libs.zip *
cd ../..

mkdir -p bin-lib
mvn install:install-file -Dfile=target/$JAR -DgroupId=com.tokera -DartifactId=ate -Dversion=$VERSION -Dpackaging=jar -DlocalRepositoryPath=bin-lib -DpomFile=pom.xml
zip -r repo.zip bin-lib
