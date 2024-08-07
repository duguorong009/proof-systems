#
# list versions of mdbook and mdbook plugins
#

MDBOOK_VERSION = 0.4.35
MDBOOK_KATEX_VERSION = 0.3.8
MDBOOK_ADMONISH_VERSION = 1.14.0
MDBOOK_LINKCHECK_VERSION = 0.7.7
MDBOOK_MERMAID_VERSION = 0.12.6
MDBOOK_TOC_VERSION = 0.14.1


all: deps check build serve

#
# Installs the dependencies required to serve or build the book
#

deps:
	cargo install --locked "mdbook@$(MDBOOK_VERSION)"
	cargo install --locked "mdbook-admonish@$(MDBOOK_ADMONISH_VERSION)"
	cargo install --locked "mdbook-katex@$(MDBOOK_KATEX_VERSION)"
	cargo install --locked --git https://github.com/o1-labs/mdbook-linkcheck --rev 8cccfc8fee397092ecdf1236a42871c5c980672e mdbook-linkcheck
	cargo install --locked "mdbook-mermaid@$(MDBOOK_MERMAID_VERSION)"
	cargo install --locked "mdbook-toc@$(MDBOOK_TOC_VERSION)"

#
# Checks if your installed dependencies match what we've listed above
#

define check_version
	@(cargo install --list | grep "$(1) v" | cut -d " " -f 2 | tr -d ':' | grep -Fx "v$(2)") || (echo "your $(1) version needs to be $(2)" && exit 1)
endef

check:
	$(call check_version,mdbook,$(MDBOOK_VERSION))
	$(call check_version,mdbook-admonish,$(MDBOOK_ADMONISH_VERSION))
	$(call check_version,mdbook-katex,$(MDBOOK_KATEX_VERSION))
	$(call check_version,mdbook-linkcheck,$(MDBOOK_LINKCHECK_VERSION))
	$(call check_version,mdbook-mermaid,$(MDBOOK_MERMAID_VERSION))
	$(call check_version,mdbook-toc,$(MDBOOK_TOC_VERSION))

#
# Builds the book
#

build: check
	mdbook build

#
# Serves the book locally
#

serve: check
	mdbook serve --open

#
# Cleans the generated artefacts
#

clean:
	mdbook clean
