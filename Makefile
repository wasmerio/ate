build:
	./build.sh

inside:
	./compile.sh

clean:
	mvn clean

distclean: clean
	rm -r -f .m2
