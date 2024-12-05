# Docker container

Notes on building images:

- We build [multi-platform images](https://docs.docker.com/build/building/multi-platform/). To do this, create a multi-arch builder:

  ```sh
  just docker-create-builder [name]
  ```

- Since we'll be building image for multiple platforms in parallel, in experience it's necessary to increase dockerd memory limit to 16 GB. This can be done in Docker Desktop > Settings > Resources.

- Typically, we build an image with `--load` flag, which adds it to the local image registry. However, this is not supported for multi-target images. Instead, the commands `just docker-build-{optimizer,devnet}` uses `--push` flag, meaning it pushes to Docker Hub automatically if build succeeds. It's recommended to build for a single target locally (with `--load`) first, test it, then use the `just` command.
