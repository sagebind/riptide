.PHONY: docs
docs: target/docs/index.html $(patsubst docs/%.adoc,target/docs/%.html,$(wildcard docs/*.adoc))

target/docs/index.html: docs/index.html
	@mkdir -p target/docs
	cp $^ $@

target/docs/%.html: docs/%.adoc
	@mkdir -p target/docs
	asciidoctor -b html -o $@ $^
