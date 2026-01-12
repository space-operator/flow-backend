FROM docker.io/denoland/deno:debian
EXPOSE 80
WORKDIR /app
USER deno
COPY . .
RUN deno cache main.ts
CMD ["run", "--allow-net", "main.ts"]
