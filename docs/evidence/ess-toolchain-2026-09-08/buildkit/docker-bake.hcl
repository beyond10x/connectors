group "default" {
  targets = ["gitlab"]
}

target "gitlab" {
  context = "."
  dockerfile = "Dockerfile.ess"
  target = "service"
  platforms = ["linux/amd64"]
}

