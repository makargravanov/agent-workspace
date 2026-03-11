# syntax=docker/dockerfile:1.7

FROM node:22-alpine

WORKDIR /app

COPY web/package.json web/package-lock.json ./

RUN --mount=type=cache,id=agent-workspace-npm-cache,sharing=locked,target=/root/.npm \
	npm ci

COPY web/ ./

EXPOSE 5173

CMD ["npm", "run", "dev", "--", "--host", "0.0.0.0", "--port", "5173"]
