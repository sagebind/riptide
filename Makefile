.PHONY: docs publish-docs

docs: $(patsubst docs/%.adoc,target/docs/%.html,$(wildcard docs/*.adoc))

publish-docs: docs
	scripts/publish-docs.sh

target/docs/%.html: docs/%.adoc
	@mkdir -p target/docs
	asciidoctor -b html -o $@ $^
