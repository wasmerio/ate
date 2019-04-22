build:
	./build.sh

inside: compile package

clean:
	mvn clean || true

compile:
	mvn compile

package:
	[ -e target/ate-0.1.jar ] && rm -f target/ate-0.1.jar || true
	mkdir -p /maven
	./compile.sh
	echo "Hash: $(cat target/ate-0.1.jar | md5sum)"
	cd target/lib; zip -r ../libs.zip *
