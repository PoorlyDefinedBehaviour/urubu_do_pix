# Running the bot

Set up the development environment:

```
docker-compose -f docker-compose-dev.yaml up -d
```

## TODO

When you mention someone, the conversation bot uses the discord id instead of the user name, fix it.

Retry google translate call when it returns an unexpected response. (see if we are being rate limited)

The chatbot api returns an error sometimes, maybe we can just wait a few ms and retry.
response="{\"error\":{\"code\":500,\"message\":\"Request to model backend failed: Expecting value: line 1 column 1 (char 0)\"}}" error=Error("missing field `data`", line: 1, column: 109)
