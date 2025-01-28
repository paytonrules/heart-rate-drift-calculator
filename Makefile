ifneq (,$(wildcard ./.env))
    include .env
    export
endif

.PHONY: watch web dev lambda-build lambda-deploy

# Remember to check for STRAVA_CLIENT_ID and STRAVA_CLIENT_SECRET
watch:
	cd functions/strava-oauth-exchange && cargo lambda watch

web:
	cd web && trunk serve

dev:
	make -j 2 web watch

lambda-build:
	cd functions/strava-oauth-exchange && cargo lambda build

lambda-deploy: lambda-build
	cd functions/strava-oauth-exchange && cargo lambda deploy strava-oauth-exchange --enable-function-url
