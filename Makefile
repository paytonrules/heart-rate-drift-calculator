ifneq (,$(wildcard ./.env))
    include .env
    export
endif

.PHONY: serve lambda-build lambda-deploy lambda-serve web-serve

lambda-build:
	cd functions/strava-oauth-exchange && cargo lambda build

lambda-deploy: lambda-build
	cd functions/strava-oauth-exchange && cargo lambda deploy strava-oauth-exchange --enable-function-url

# Remember to check for STRAVA_CLIENT_ID and STRAVA_CLIENT_SECRET
lambda-serve:
	cd functions/strava-oauth-exchange && cargo lambda watch

web-serve:
	cd web && trunk serve

serve:
	make -j 2 web-serve lambda-serve


