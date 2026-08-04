# Astro's static + NGINX recipe: https://docs.astro.build/en/recipes/docker/
#
# The recipe is written for npm; the docs note you adjust the commands for other
# package managers, and this repo uses Bun exclusively for frontend dependencies
# (there is no npm or pnpm lockfile to install from). So the build stage is Bun
# and everything else follows the recipe as published.

# Pinned rather than floating on `oven/bun:1.3`: that tag currently resolves to
# bun 1.3.14, which dies with "Fail extracting tarball for astro" and fails the
# build outright. 1.3.10 installs all 436 packages cleanly and matches the
# version this project is developed against.
FROM oven/bun:1.3.10 AS build
WORKDIR /app
COPY package.json bun.lock ./
RUN bun install --frozen-lockfile
COPY . .
RUN bun run build

FROM nginx:alpine AS runtime
COPY ./nginx/nginx.conf /etc/nginx/nginx.conf
COPY --from=build /app/dist /usr/share/nginx/html
# 80, not the recipe's 8080. See the note at the top of nginx/nginx.conf.
EXPOSE 80
