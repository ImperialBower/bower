# Bower playable-chapter environment (spec § 15) on top of the spark4 playbook.
# spark4 already carries JupyterLab, Rust/cargo, evcxr_jupyter, maturin, PyO3.
FROM folkengine/spark4:latest

USER root
RUN pip install --no-cache-dir "pkcore.py==0.11.0" nbformat nbclient
COPY bower_ipynb.py /usr/local/bin/bower-ipynb
RUN chmod +x /usr/local/bin/bower-ipynb
USER ${NB_UID}

# book/ and notebooks/ are mounted by docker-compose; see Makefile targets.
