build:
	./build.sh

inside:
	./compile.sh

clean:
	rm -r -f target

distclean: clean
	rm -r -f .m2
