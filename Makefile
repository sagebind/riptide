.PHONY: docs

docs: $(patsubst docs/%.adoc,target/docs/%.html,$(wildcard docs/*.adoc))

target/docs/%.html: docs/%.adoc
	@mkdir -p target/docs
	asciidoctor -b html -o $@ $^
