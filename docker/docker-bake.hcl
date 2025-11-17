group "default" {
  targets = ["cross-builder-amd64", "cross-builder-arm64",
    "native-builder-amd64", "native-builder-arm64"]
}

target "cross-builder-arm64" {
  context = "cross-builder-arm64"
  dockerfile = "Dockerfile"
  tags = ["ghcr.io/left-curve/left-curve/cross-builder-arm64:latest"]
  platforms = ["linux/amd64", "linux/arm64"]
  push = true
}

target "cross-builder-amd64" {
  context = "cross-builder-amd64"
  dockerfile = "Dockerfile"
  tags = ["ghcr.io/left-curve/left-curve/cross-builder-amd64:latest"]
  platforms = ["linux/arm64", "linux/amd64"]
  push = true
}

target "native-builder-amd64" {
  context = "native-builder"
  dockerfile = "Dockerfile"
  args = {
    DEBIAN_ARCH = "x86_64-linux-gnu"
  }
  tags = ["ghcr.io/left-curve/left-curve/native-builder:amd64"]
  platforms = ["linux/amd64"]
  push = true
  provenance = false
}

target "native-builder-arm64" {
  context = "native-builder"
  dockerfile = "Dockerfile"
  args = {
    DEBIAN_ARCH = "aarch64-linux-gnu"
  }
  tags = ["ghcr.io/left-curve/left-curve/native-builder:arm64"]
  platforms = ["linux/arm64"]
  push = true
  provenance = false
}
