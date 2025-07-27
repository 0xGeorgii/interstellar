FROM node:24.4-slim AS builder
WORKDIR /app

COPY package*.json ./
RUN npm install --legacy-peer-deps
RUN npm audit fix --force
COPY . .
RUN npm run build

FROM nginx:stable-alpine-perl
RUN rm -rf /usr/share/nginx/html/*

COPY --from=builder /app/dist /usr/share/nginx/html
COPY --from=builder /app/nginx.conf /etc/nginx/nginx.conf
COPY --from=builder /app/static /usr/share/nginx/html/static

CMD ["/bin/sh",  "-c",  "exec nginx -g 'daemon off;'"]
EXPOSE 80
