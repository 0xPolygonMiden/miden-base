#!/usr/bin/env bash
set -euo pipefail

virtualenv venv
source venv/bin/activate
pip3 install -r docs_requirements.txt
cd ..
mkdocs serve --strict
