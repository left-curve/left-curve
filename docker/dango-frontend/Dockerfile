FROM node:24-alpine AS builder

# Install pnpm
RUN corepack enable pnpm

COPY ./sdk /app/sdk
COPY ./ui /app/ui
COPY ./package.json /app/package.json
COPY ./pnpm-lock.yaml /app/pnpm-lock.yaml
COPY ./pnpm-workspace.yaml /app/pnpm-workspace.yaml
COPY ./turbo.json /app/turbo.json
COPY ./.npmrc /app/.npmrc

WORKDIR /app

# Need GITHUB_TOKEN for private packages
ARG GITHUB_TOKEN
ENV GITHUB_TOKEN=$GITHUB_TOKEN

# CONFIG_ENVIRONMENT: ${{ github.ref_name == 'ui/testnet' && 'test' || github.ref_name == 'ui/release' && 'prod' || 'dev' }}
# PUBLIC_WEBRTC_URI: ${{ secrets.PUBLIC_WEBRTC_URI }}
# PUBLIC_SENTRY_DSN: ${{ secrets.PUBLIC_SENTRY_DSN }}
# SENTRY_PROJECT: ${{ secrets.SENTRY_PROJECT }}
# SENTRY_ORG: ${{ secrets.SENTRY_ORG }}
# SENTRY_AUTH_TOKEN: ${{ secrets.SENTRY_AUTH_TOKEN }}

# Install dependencies
RUN pnpm i --frozen-lockfile

# RUN pnpm add -D @rollup/rollup-linux-arm64-gnu && pnpm run build

# Build the website
RUN pnpm build:portal-web

# Production stage
FROM nginx:alpine

LABEL org.opencontainers.image.description="Dango frontend docker image"
# LABEL org.opencontainers.image.revision=$GIT_COMMIT
LABEL org.opencontainers.image.source="https://github.com/left-curve/left-curve"

COPY --from=builder /app/ui/portal/web/build /usr/share/nginx/html
COPY docker/dango-frontend/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
