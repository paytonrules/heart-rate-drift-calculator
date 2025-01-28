.PHONY: dev lambda web

lambda:
	cd functions/strava-oauth-exchange && cargo lambda watch

web:
	cd web && trunk serve

dev:
	make -j 2 web lambda

# cargo lambda deploy strava-oauth-exchange --enable-function-url
