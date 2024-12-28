patch:
	@bumpversion --commit --tag version

build-status:
	@curl -L \
		-H "Accept: application/vnd.github+json" \
		-H "X-GitHub-Api-Version: 2022-11-28" \
		https://api.github.com/repos/kunansy/NotesReminder/actions/runs \
		| jq '[.workflow_runs | .[] | select(.name == "Build docker image")] | .[0] | .name,.display_title,.status,.conclusion'

CURRENT_TAG := $(shell git describe --tags --abbrev=0)
LAST_TAG := $(shell git describe --tags --abbrev=0 HEAD^)
IMAGE_LINE := $(shell cat docker-compose.yml | grep -n "image: kunansy/notes_reminder" | cut -f1 -d:)

deploy:
	@echo "${LAST_TAG} -> ${CURRENT_TAG}"
	@ssh tracker "cd notes_reminder; sed -i -E '${IMAGE_LINE} s/:[0-9.]+/:${CURRENT_TAG}/' docker-compose.yml; docker compose up -d --build --force-recreate; sleep 2; docker ps --filter name=notes-reminder --format json | jq '.Image,.State,.Status'"
