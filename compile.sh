gradle build

##!/bin/bash -e
#rm -f target/*.jar || true
#rm -r -f bin-lib || true
#[ -e repo.zip ] && rm -f repo.zip || true
#[ -e target/libs.zip ] && rm -f target/libs.zip || true
#
#mkdir -p /maven
#export MAVEN_OPTS="-Dmaven.repo.local=/maven"
#
#mvn -T 2C compile test package
#
#JAR=$(basename target/*.jar)
#[ -z "$JAR" ] && echo "No JAR file exists" && exit 1
#VERSION=$(echo $JAR | cut -d "-" -f2 | sed 's|\.jar||g')
#[ -z "$VERSION" ] && echo "Version could not be determined" && exit 1
#
#mkdir -p bin-lib
#mvn install:install-file -Dfile=ate-deps/pom.xml -DgroupId=com.tokera -DartifactId=ate-deps -Dversion=$VERSION -Dpackaging=pom -DlocalRepositoryPath=bin-lib -DpomFile=ate-deps/pom.xml
#mvn install:install-file -Dfile=target/$JAR -DgroupId=com.tokera -DartifactId=ate -Dversion=$VERSION -Dpackaging=jar -DlocalRepositoryPath=bin-lib -DpomFile=pom.xml
#
#pushd bin-lib >/dev/null
#zip -r ../repo.zip *
#popd >/dev/null
