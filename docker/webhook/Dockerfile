FROM docker.io/denoland/deno:2.6.4
EXPOSE 80
WORKDIR /app
USER deno
COPY . .
RUN deno install --entrypoint main.ts
CMD ["run", "--allow-net", "main.ts"]
