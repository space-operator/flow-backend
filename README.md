# flow-backend
Space Operator Backend

## Running a guest server

You can run a local instance of flow-server and have it push results to our site's database.
Set required values in [guest.toml](https://github.com/space-operator/flow-backend/blob/main/guest.toml)
and run with:
```bash
RUST_LOG=info cargo run -p flow-server -- guest.toml
```

This server can be used to run flows belonging to you. Toggle "Remote 🌐" button in flow editor to switch to `localhost` server.
Please note that some browsers such as Brave will block requests to `localhost` by default, disable protection if you encounter network errors.
```print```
