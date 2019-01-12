workflow "Main" {
  on = "push"
  resolves = ["Publish docs"]
}

action "Test" {
  uses = "docker://rust"
  args = "cargo test"
}

action "Master" {
  uses = "actions/bin/filter@b2bea0749eed6beb495a8fa194c071847af60ea1"
  needs = ["Test"]
  args = "branch master"
}

action "Publish docs" {
  uses = "docker://asciidoctor/docker-asciidoctor"
  needs = ["Master"]
  args = ["sh", "-c", "apk --no-cache add git && make publish-docs"]
  secrets = ["GITHUB_TOKEN"]
}
