#!/bin/bash -e
rm -f target/*.jar || true
rm -r -f bin-lib || true
[ -e repo.zip ] && rm -f repo.zip || true
[ -e target/libs.zip ] && rm -f target/libs.zip || true

mvn -T 2C -Dmaven.test.skip=true compile package

JAR=$(basename target/*-SNAPSHOT.jar)
[ -z "$JAR" ] && echo "No JAR file exists" && exit 1
VERSION=$(echo $JAR | cut -d "-" -f2 | sed 's|\.jar||g')
[ -z "$VERSION" ] && echo "Version could not be determined" && exit 1

mkdir -p bin-lib
mvn install:install-file -Dfile=ate-deps/pom.xml -DgroupId=com.tokera -DartifactId=ate-deps -Dversion=$VERSION -Dpackaging=pom -DpomFile=ate-deps/pom.xml
mvn install:install-file -Dfile=target/$JAR -DgroupId=com.tokera -DartifactId=ate -Dversion=$VERSION -Dpackaging=jar -DpomFile=pom.xml

mvn install:install-file -Dfile=ate-deps/pom.xml -DgroupId=com.tokera -DartifactId=ate-deps -Dversion=$VERSION -Dpackaging=pom -DpomFile=ate-deps/pom.xml -DlocalRepositoryPath=../tokera/tokapi/.m2
mvn install:install-file -Dfile=target/$JAR -DgroupId=com.tokera -DartifactId=ate -Dversion=$VERSION -Dpackaging=jar -DpomFile=pom.xml -DlocalRepositoryPath=../tokera/tokapi/.m2
