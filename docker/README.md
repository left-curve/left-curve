# Docker build images

If you need to rebuild docker images to build on our CI (like a Rust update),
you can run:

`just docker-build-builder-images`

This will build and push docker images used by our CI jobs.
