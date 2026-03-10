FROM node:24-alpine AS builder
ENV NO_UPDATE_NOTIFIER=true
WORKDIR /build
RUN --mount=type=bind,source=package.json,target=package.json \
    --mount=type=bind,source=package-lock.json,target=package-lock.json \
    --mount=type=cache,target=/root/.npm \
    npm ci --no-audit --no-fund --include=dev

COPY --link src ./src
COPY --link public ./public
COPY --link *.ts *.json index.html ./

RUN --mount=type=cache,target=/root/.npm \
    npm run build

FROM nginx:1.28.2-alpine

# Fix CVE-2025-58050 in pcre2 (require >=10.46)
RUN apk add --no-cache 'pcre2>=10.46-r0'

LABEL org.opencontainers.image.description="GraphiQL image"
LABEL org.opencontainers.image.source=https://github.com/left-curve/left-curve

# Default can be overridden at `docker run -e GRAPHQL_ENDPOINT=...`
ENV GRAPHQL_ENDPOINT=http://localhost:4000/graphql

# Copy built assets to nginx docroot
COPY --link --from=builder /build/dist /usr/share/nginx/html

# Add entrypoint to generate runtime config from env
COPY --link docker-entrypoint.sh /docker-entrypoint.sh
RUN chmod +x /docker-entrypoint.sh

EXPOSE 80

ENTRYPOINT ["/docker-entrypoint.sh"]
