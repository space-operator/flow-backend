FROM docker.io/denoland/deno:2.6.4
EXPOSE 80
WORKDIR /app
RUN chown deno:deno /app
COPY --chown=deno:deno deno.json deno.lock main.ts ./
USER deno
RUN deno install --entrypoint main.ts
CMD ["run", "--allow-net", "main.ts"]
