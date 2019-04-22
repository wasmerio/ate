#!/bin/bash -e

# If the cache is empty then it will be removed
rmdir .m2 2>/dev/null || true

# Copy all the maven files we already have over to the maven folder
if [ ! -d .m2 ]; then
  mkdir -p .m2
  docker run -u $(id -u ${USER}):$(id -g ${USER}) -v $(pwd)/.m2:/maven-local tokera/buildj1:latest cp -a -f -r /maven/. /maven-local/
fi

# Execute the build (using the maven cache)
docker run -u $(id -u ${USER}):$(id -g ${USER}) -v $(pwd):/build -v $(pwd)/.m2:/maven -w /build tokera/buildj1:latest make inside
