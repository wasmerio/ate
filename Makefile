all:
	gradle build
	gradle jar

inside:
	./compile.sh

clean:
	mvn clean

distclean: clean
	rm -r -f .m2

local:
	./compile_local.sh
