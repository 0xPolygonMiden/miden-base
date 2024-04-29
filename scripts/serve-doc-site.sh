#!/usr/bin/env bash
set -euo pipefail

virtualenv venv
source venv/bin/activate
pip3 install -r scripts/docs_requirements.txt
mkdocs serve --strict
