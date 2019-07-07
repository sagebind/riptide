workflow "build" {
  on = "push"
  resolves = ["test", "publish-docs"]
}

action "test" {
  uses = "docker://rust"
  args = "cargo test"
}

action "master-branch" {
  uses = "actions/bin/filter@b2bea0749eed6beb495a8fa194c071847af60ea1"
  args = "branch master"
}

action "docs" {
  uses = "docker://asciidoctor/docker-asciidoctor"
  args = "make docs"
}

action "publish-docs" {
  uses = "maxheld83/ghpages@v0.2.1"
  needs = ["master-branch", "docs"]
  env = {
    BUILD_DIR = "target/docs/"
  }
  secrets = ["GH_PAT"]
}
