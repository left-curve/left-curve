group "default" {
  targets = ["cross-builder-amd64", "cross-builder-arm64", "native-builder-amd64", "native-builder-arm64"]
}

target "cross-builder-amd64" {
  context = "docker/cross-builder-amd64"
  dockerfile = "Dockerfile"
  tags = ["ghcr.io/left-curve/left-curve/cross-builder:amd64"]
  platforms = ["linux/amd64"]
  push = true
  provenance = false
  cache-from = [
    # "type=local,src=/tmp/.buildx-cache/cross-builder-amd64",
    "type=registry,ref=ghcr.io/left-curve/left-curve/cross-builder:amd64-cache"
  ]
  cache-to = [
    # "type=local,dest=/tmp/.buildx-cache/cross-builder-amd64",
    "type=registry,ref=ghcr.io/left-curve/left-curve/cross-builder:amd64-cache,mode=max"
  ]
}

target "cross-builder-arm64" {
  context = "docker/cross-builder-arm64"
  dockerfile = "Dockerfile"
  tags = ["ghcr.io/left-curve/left-curve/cross-builder:arm64"]
  platforms = ["linux/arm64"]
  push = true
  provenance = false
  cache-from = [
    # "type=local,src=/tmp/.buildx-cache/cross-builder-arm64",
    "type=registry,ref=ghcr.io/left-curve/left-curve/cross-builder:arm64-cache"
  ]
  cache-to = [
    # "type=local,dest=/tmp/.buildx-cache/cross-builder-arm64",
    "type=registry,ref=ghcr.io/left-curve/left-curve/cross-builder:arm64-cache,mode=max"
  ]
}

target "native-builder-amd64" {
  context = "docker/native-builder"
  dockerfile = "Dockerfile"
  args = {
    DEBIAN_ARCH = "x86_64-linux-gnu"
  }
  tags = ["ghcr.io/left-curve/left-curve/native-builder:amd64"]
  platforms = ["linux/amd64"]
  push = true
  provenance = false
  cache-from = [
    # "type=local,src=/tmp/.buildx-cache/native-builder-amd64",
    "type=registry,ref=ghcr.io/left-curve/left-curve/native-builder:amd64-cache"
  ]
  cache-to = [
    # "type=local,dest=/tmp/.buildx-cache/native-builder-amd64",
    "type=registry,ref=ghcr.io/left-curve/left-curve/native-builder:amd64-cache,mode=max"
  ]
}

target "native-builder-arm64" {
  context = "docker/native-builder"
  dockerfile = "Dockerfile"
  args = {
    DEBIAN_ARCH = "aarch64-linux-gnu"
  }
  tags = ["ghcr.io/left-curve/left-curve/native-builder:arm64"]
  platforms = ["linux/arm64"]
  push = true
  provenance = false
  cache-from = [
    # "type=local,src=/tmp/.buildx-cache/native-builder-arm64",
    "type=registry,ref=ghcr.io/left-curve/left-curve/native-builder:arm64-cache"
  ]
  cache-to = [
    # "type=local,dest=/tmp/.buildx-cache/native-builder-arm64",
    "type=registry,ref=ghcr.io/left-curve/left-curve/native-builder:arm64-cache,mode=max"
  ]
}
