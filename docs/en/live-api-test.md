# Live API validation

`live_api_test` is an interactive advanced example for maxoxide users who want to validate the library against the real MAX Bot API with their own bot credentials. It complements offline tests by exercising actual authorization, transport, update delivery, messages, uploads, subscriptions, and optional chat administration.

This example performs real API calls. Use a dedicated test bot, controlled users and chats, and disposable content. Do not run destructive scenarios against a production bot or an important chat.

## Get the matching source

Cargo does not run examples from a dependency inside the consuming application. Run this example from a maxoxide source checkout matching the version used by your application:

```bash
git clone https://github.com/mammothcoding/maxoxide.git
cd maxoxide
git checkout "v<version>"
cargo run --example live_api_test
```

Replace `<version>` with the maxoxide version in your application; for example, use `git checkout v3.1.0` for version `3.1.0`. To exercise unreleased code from the default branch, omit `git checkout`.

Rust 1.85 or newer is required.

## Credentials

The Bot API token is required. The webhook secret is optional:

- `MAX_BOT_TOKEN`: token of the dedicated test bot;
- `MAX_WEBHOOK_SECRET`: secret used by the webhook subscription being tested or temporarily disabled.

If a variable is absent, the example asks for it interactively and masks the input. An empty webhook-secret prompt skips that optional value. Secrets are not accepted as command-line arguments.

For an interactive run without placing a token in the command line:

```bash
cargo run --example live_api_test
```

If you provide credentials through the environment, load them through your usual protected mechanism rather than committing them to a shell script or repository file. Unset temporary variables after the run.

## Configuration

At startup, select English or Russian and choose an update transport:

- `long_polling`: checks typed, raw, and filtered polling. Active webhook subscriptions prevent long polling, so the example can temporarily unsubscribe them and restore them afterward.
- `webhook`: starts a minimal local HTTP receiver and consumes updates delivered by MAX. A public HTTPS URL must route to the prompted local listen address, directly or through a controlled reverse proxy/tunnel.

The remaining prompts are optional unless the current scenario needs them:

- bot URL shown during manual private-chat activation;
- public channel link for `get_chat_by_link`;
- channel post ID for the optional comments phase;
- public webhook URL and webhook secret;
- local file, image, video, and audio paths;
- delay between API requests, 400 ms by default;
- polling timeout, 5 seconds by default.

When the private-chat phase starts, send a new `/live` message to the bot. A message sent before the waiting prompt is ignored when the update backlog is cleared.

## Covered scenarios

The example checks or offers interactive checks for:

- bot authorization and information;
- long-polling or webhook update delivery;
- channel lookup by public link;
- chat- and user-addressed text, Markdown, structured messages, and link-preview options;
- keyboards, callbacks, message buttons, contact/location requests, clipboard, Mini App, and chat buttons;
- message retrieval, editing, and deletion;
- comment listing and optional confirmed creation, retrieval, editing, and deletion;
- upload URLs, streamed file/byte uploads, and optional image/video/audio send helpers;
- webhook subscriptions and optional temporary command-menu replacement;
- optional group metadata, members, administrators, sender actions, pinning, title rollback, leaving, and deletion.

Optional scenarios without the required input or manual confirmation are reported as `SKIP`. The final summary lists every `PASS`, `FAIL`, and `SKIP`; one or more failures produce a non-zero process exit status.

## Side effects and cleanup

The example asks before optional or destructive operations, but confirmed actions are real:

- test messages and uploaded attachments are created and may remain in chats;
- webhook subscriptions may be removed and recreated;
- the bot command menu may be replaced temporarily;
- a group title may be changed and rolled back;
- administrator state may be changed;
- a temporary comment is created, edited, and deleted when the comments probe is confirmed;
- `delete_chat` and `leave_chat` are destructive and cannot be rolled back by maxoxide;
- a chat button may create a real chat, after which the example asks whether to delete it, leave it, or keep it.

The example restores captured command, webhook, and title state where possible and removes its generated temporary upload file. Restoration is best-effort: interruption, lost connectivity, insufficient permissions, or an API failure can leave state changed. Review the bot commands, webhook subscriptions, created chats, group membership, and test messages after every run.

MAX does not return webhook secrets from `get_subscriptions`. If an existing subscription uses a secret, provide the matching `MAX_WEBHOOK_SECRET` before allowing the example to disable it; otherwise an identical restoration cannot be guaranteed.

## Limitations

- `Bot::add_members` and the formerly reversible member-removal probe are reported as `SKIP` because MAX removed the add-members endpoint on September 30, 2026.
- The experimental Digital ID partner integration is outside this example and requires separate credentials and onboarding schemas.
- A successful run validates the selected scenarios for that bot, account, client, and point in time. It does not prove production capacity, availability, or correctness of application-specific handlers.
- Treat console output as diagnostic data and review it before sharing. Do not publish credentials, personal data, chat identifiers, or webhook details.
