# The gate

One command has to mean "check everything". In these books that command is
`make ayce` — *all your code, evaluated* — and its meaning never varies from
project to project. What varies is what it depends on, and that list grows as
the book does.

<!-- bower repo="hello-playbook" step="makefile" file="Makefile" msg="build: one command runs everything" -->

```makefile
.DEFAULT_GOAL := ayce

# bower:begin help
# bower:end help

# bower:begin gate
GATE := build

build:
	cargo build --workspace

.PHONY: build
# bower:end gate

ayce: $(GATE)
	@echo "ayce — all your code, evaluated"

.PHONY: ayce
```

The `GATE` variable lives *inside* the region, which is the whole trick. Later
chapters replace the region wholesale and the `ayce` target picks up the new
prerequisites without ever being edited again.

<!-- bower repo="hello-playbook" step="make-help" file="Makefile" op="region" region="help" msg="build: make help lists the targets" -->

```makefile
help:
	@echo "ayce   run the whole gate"
	@echo "build  compile the workspace"
	@echo "help   this list"

.PHONY: help
```

`op="region"` replaces the text between the two `help` markers and leaves the
rest of the file alone. The page shows you five lines; the repository gets the
whole Makefile. Note that the block does *not* repeat the marker lines — they
stay in the file, waiting for the next edit.
