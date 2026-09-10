CH ?= ch01-ranks

.PHONY: build verify up local
build:      ## render book/src/$(CH).md -> notebooks/$(CH).ipynb
	docker compose run --rm bower-notebook bower-ipynb build work/book/src/$(CH).md -o work/notebooks/$(CH).ipynb --config work/book/bower.toml
verify:     ## execute the notebook; play cells must pass / raise as declared
	docker compose run --rm bower-notebook bower-ipynb verify work/notebooks/$(CH).ipynb
up:         ## JupyterLab at http://localhost:8888
	docker compose up
local:      ## same thing without Docker (pip install pkcore.py nbformat nbclient ipykernel)
	python3 bower_ipynb.py build book/src/$(CH).md -o notebooks/$(CH).ipynb && python3 bower_ipynb.py verify notebooks/$(CH).ipynb
