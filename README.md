# Tauri Template W7

This project uses Tauri v2 with GitHub Releases as the updater backend.

## Updater setup

To make auto-updates work, the project needs four pieces wired together:

1. The Tauri updater plugin must be enabled in the Rust app.
2. `src-tauri/tauri.conf.json` must contain the updater `pubkey` and the `latest.json` endpoint.
3. Release builds must be signed with the private key.
4. GitHub Actions must publish the signed bundles, signatures, and `latest.json` to the GitHub Release.

The repository is already configured for items 1, 2, and 4. What each maintainer needs to provide is the signing key secret.

## Generate signing keys

Generate the updater signing keypair locally:

```bash
mkdir -p ~/.tauri
pnpm tauri signer generate --ci -w ~/.tauri/myapp.key
chmod 600 ~/.tauri/myapp.key
```

This creates:

- `~/.tauri/myapp.key`: private key, keep this secret.
- `~/.tauri/myapp.key.pub`: public key, safe to share.

If you want password protection for the private key, generate it with:

```bash
pnpm tauri signer generate --ci -w ~/.tauri/myapp.key --password 'your-password'
chmod 600 ~/.tauri/myapp.key
```

## Configure the app

The updater public key is stored in `src-tauri/tauri.conf.json` under:

- `plugins.updater.pubkey`
- `plugins.updater.endpoints`

This project uses GitHub Releases as the static updater endpoint:

```text
https://github.com/fikryfahrezy/tauri-template-w7/releases/latest/download/latest.json
```

`bundle.createUpdaterArtifacts` is enabled, so Tauri generates the update artifacts and signatures during release builds.

## Configure GitHub Actions secrets

Add these repository secrets in GitHub:

- `TAURI_SIGNING_PRIVATE_KEY`: the full contents of `~/.tauri/myapp.key`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: the private key password, if you created one

The workflow at `.github/workflows/build.yaml` already passes these values into `tauri-apps/tauri-action` and explicitly enables:

- `uploadUpdaterJson: true`
- `uploadUpdaterSignatures: true`

That makes the action upload:

- platform bundles
- `.sig` signature files
- `latest.json`

## Publish an update

1. Bump the app version in `src-tauri/tauri.conf.json` and `package.json` if needed.
2. Push to `main` or run the build workflow manually.
3. Wait for the workflow to finish successfully.
4. Open the GitHub Release created by `tauri-apps/tauri-action`.
5. Confirm the release contains `latest.json` and the signed platform artifacts.

The updater endpoint always reads the latest non-draft, non-prerelease GitHub Release from:

```text
releases/latest/download/latest.json
```

If you want a release to be ignored by the updater, keep it as a draft or prerelease.

## Add in-app update checks

The backend plugin and permissions are enabled, and the frontend now exposes a manual updater flow.

Current behavior:

- The app only checks for updates when the user clicks `Check for updates`.
- The app skips updater checks while running in dev mode.
- If the machine is offline, the app shows a connection message and does not interrupt the user.
- If an update is found, the app shows an `Install` action and waits for the user to confirm.
- When the user chooses to install, the app downloads, installs, and relaunches.

The implemented frontend flow is:

```ts
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

const update = await check();

if (update) {
	await update.downloadAndInstall();
	await relaunch();
}
```

The updater implementation lives in `src/App.vue`.

## Verification checklist

- The app includes `tauri-plugin-updater` and initializes it in Rust.
- `src-tauri/tauri.conf.json` contains the correct public key.
- The GitHub repository has the signing key secret.
- The GitHub Release contains `latest.json`.
- The app can successfully call the updater API and install a newer signed release.